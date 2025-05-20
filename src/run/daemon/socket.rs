use std::{
    error::Error,
    fs::{self},
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream, unix::SocketAddr},
};

#[allow(unused)]
pub struct AsyncUnixSocket {
    socketpath: String,
    authgroup: String,
    listener: Option<UnixListener>,
    stream: Option<UnixStream>,
}

#[cfg(not(test))]
fn get_group_id(group_name: &str) -> Result<u32, String> {
    let c_group = std::ffi::CString::new(group_name).map_err(|e| format!("{e}"))?;

    unsafe {
        let grp_ptr = libc::getgrnam(c_group.as_ptr());
        if grp_ptr.is_null() {
            Err(format!("Group '{group_name}' not found"))
        } else {
            Ok((*grp_ptr).gr_gid)
        }
    }
}

#[cfg(not(test))]
fn set_permissions(socketpath: &str, authgroup: &str, gid: u32) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let c_path = std::ffi::CString::new(socketpath).map_err(|e| format!("invalid path: {e}"))?;

    unsafe {
        if libc::chown(c_path.as_ptr(), u32::MAX, gid as libc::gid_t) != 0 {
            return Err(format!(
                "could not change group ownership: {} - do you have permissions for group '{}'?",
                std::io::Error::last_os_error(),
                authgroup
            ));
        }
    }

    fs::set_permissions(socketpath, fs::Permissions::from_mode(0o660)).map_err(|e| format!("could not set permissions: {e}"))
}

impl AsyncUnixSocket {
    pub fn new(socketpath: &str, authgroup: &str) -> Result<Self, String> {
        if fs::metadata(socketpath).is_ok() {
            let _ = fs::remove_file(socketpath);
        }

        let listener = match UnixListener::bind(socketpath) {
            Ok(listener) => listener,
            Err(e) => {
                return Err(format!("could not bind to unix socket at path {socketpath}: {e}"));
            }
        };

        #[cfg(not(test))]
        let gid = get_group_id(authgroup)?;
        #[cfg(not(test))]
        if let Err(e) = set_permissions(socketpath, authgroup, gid) {
            return Err(format!("could not create UNIX socket at path {socketpath}: {e}"));
        }

        Ok(Self {
            socketpath: socketpath.to_string(),
            authgroup: authgroup.to_owned(),
            listener: Some(listener),
            stream: None,
        })
    }

    pub fn stream(&self) -> &Option<UnixStream> {
        &self.stream
    }

    pub async fn accept(&mut self) -> Result<(UnixStream, SocketAddr), Box<dyn Error + Send + Sync>> {
        if let Some(listener) = &self.listener {
            match listener.accept().await {
                Ok((stream, addr)) => Ok((stream, addr)),
                Err(e) => Err(format!("accept: {e}").into()),
            }
        } else {
            Err("listener not available".to_string().into())
        }
    }

    pub async fn read_line(&mut self, line: &mut String) -> Result<usize, Box<dyn Error + Send + Sync>> {
        if self.stream.is_none() {
            self.accept().await.unwrap();
        }

        if let Some(ref mut stream) = self.stream {
            let mut reader = BufReader::new(stream);
            reader.read_line(line).await.map_err(Box::<dyn Error + Send + Sync>::from)
        } else {
            Err(Box::<dyn Error + Send + Sync>::from("no connection established"))
        }
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<(), Box<dyn Error + Send + Sync>> {
        if self.stream.is_none() {
            self.accept().await.unwrap();
        }

        if let Some(ref mut stream) = self.stream {
            stream.write_all(data).await.map_err(|e| format!("write error: {e}")).unwrap();
            stream.flush().await.map_err(|e| format!("flush error: {e}")).unwrap();
            Ok(())
        } else {
            Err(Box::<dyn Error + Send + Sync>::from("no connection established"))
        }
    }
}
