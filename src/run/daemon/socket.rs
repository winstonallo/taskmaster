use std::{
    error::Error,
    ffi::CString,
    fs::{self},
    os::unix::fs::PermissionsExt,
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
};

use libc::{chown, getgrnam, gid_t};

#[allow(unused)]
pub struct AsyncUnixSocket {
    socketpath: String,
    authgroup: String,
    listener: Option<UnixListener>,
    stream: Option<UnixStream>,
}

fn get_group_id(group_name: &str) -> Result<u32, String> {
    let c_group = CString::new(group_name).map_err(|e| format!("{}", e))?;

    unsafe {
        let grp_ptr = getgrnam(c_group.as_ptr());
        if grp_ptr.is_null() {
            Err(format!("group '{}' not found", group_name))
        } else {
            Ok((*grp_ptr).gr_gid)
        }
    }
}

fn set_permissions(socketpath: &str, authgroup: &str) -> Result<(), String> {
    let gid = get_group_id(authgroup)?;
    let c_path = CString::new(socketpath).map_err(|e| format!("invalid path: {}", e))?;

    unsafe {
        if chown(c_path.as_ptr(), u32::MAX, gid as gid_t) != 0 {
            return Err(format!(
                "could not change group ownership: {} - do you have permissions for group '{}'?",
                std::io::Error::last_os_error(),
                authgroup
            ));
        }
    }

    fs::set_permissions(socketpath, fs::Permissions::from_mode(0o660)).map_err(|e| format!("could not set permissions: {}", e))
}

impl AsyncUnixSocket {
    pub fn new(socketpath: &str, authgroup: &str) -> Result<Self, String> {
        if fs::metadata(socketpath).is_ok() {
            let _ = fs::remove_file(socketpath);
        }

        let listener = match UnixListener::bind(socketpath) {
            Ok(listener) => listener,
            Err(e) => {
                return Err(format!("could not bind to unix socket at path {}: {}", socketpath, e));
            }
        };

        #[cfg(not(test))]
        set_permissions(socketpath, authgroup)?;

        Ok(Self {
            socketpath: socketpath.to_string(),
            authgroup: authgroup.to_string(),
            listener: Some(listener),
            stream: None,
        })
    }

    pub async fn accept(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(listener) = &self.listener {
            match listener.accept().await {
                Ok((stream, _)) => {
                    self.stream = Some(stream);
                    Ok(())
                }
                Err(e) => Err(format!("accept: {e}").into()),
            }
        } else {
            Err("listener not available".to_string().into())
        }
    }

    pub async fn read_line(&mut self, line: &mut String) -> Result<usize, Box<dyn Error + Send>> {
        if self.stream.is_none() {
            self.accept().await.unwrap();
        }

        if let Some(ref mut stream) = self.stream {
            let mut reader = BufReader::new(stream);
            reader.read_line(line).await.map_err(|e| Box::new(e) as Box<dyn Error + Send>)
        } else {
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotConnected, "no connection established")) as Box<dyn Error + Send>)
        }
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<(), Box<dyn Error + Send>> {
        if self.stream.is_none() {
            self.accept().await.unwrap();
        }

        if let Some(ref mut stream) = self.stream {
            stream.write_all(data).await.map_err(|e| format!("write error: {}", e)).unwrap();
            stream.flush().await.map_err(|e| format!("flush error: {}", e)).unwrap();
            Ok(())
        } else {
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotConnected, "no connection established")) as Box<dyn Error + Send>)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_group_id_success() {
        assert_eq!(get_group_id("root").unwrap(), 0);
    }

    #[test]
    fn get_group_id_nonexisting() {
        assert!(get_group_id("randomaaaahgroup").is_err());
    }
}
