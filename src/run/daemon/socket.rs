use std::{
    ffi::CString,
    fs::{self},
    io::{ErrorKind, Read, Write},
    os::unix::{
        fs::PermissionsExt,
        net::{UnixListener, UnixStream},
    },
};

use libc::{chown, getgrnam, gid_t};

#[allow(unused)]
pub struct UnixSocket {
    path: String,
    authgroup: String,
    listener: UnixListener,
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

impl UnixSocket {
    pub fn new(path: &str, authgroup: &str) -> Result<Self, String> {
        if fs::metadata(path).is_ok() {
            let _ = fs::remove_file(path);
        }
        let listener = UnixListener::bind(path).map_err(|err| format!("could not bind to socket at path: {}: {}", path, err))?;

        listener
            .set_nonblocking(true)
            .map_err(|err| format!("failed to set non-blocking mode: {}", err))?;

        let gid = get_group_id(authgroup)?;

        let c_path = CString::new(path).map_err(|e| format!("invalid path: {}", e))?;

        unsafe {
            if chown(c_path.as_ptr(), u32::MAX, gid as gid_t) != 0 {
                return Err(format!(
                    "could not change group ownership: {} - do you have permissions for group '{}'?",
                    std::io::Error::last_os_error(),
                    authgroup
                ));
            }
        }

        fs::set_permissions(path, fs::Permissions::from_mode(0o660)).map_err(|e| format!("could not set permissions: {}", e))?;

        Ok(Self {
            path: path.to_string(),
            authgroup: authgroup.to_string(),
            listener,
            stream: None,
        })
    }

    pub fn poll(&mut self) -> Option<Vec<u8>> {
        if self.stream.is_none() {
            match self.listener.accept() {
                Ok((stream, _)) => self.stream = Some(stream),
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => return None,
                Err(e) => {
                    eprintln!("accept error: {}", e);
                    return None;
                }
            }
        }

        if let Some(ref mut stream) = self.stream {
            let mut req = Vec::new();
            match stream.read_to_end(&mut req) {
                Ok(0) => {
                    self.stream = None;
                    None
                }
                Ok(_) => Some(req),
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => None,
                Err(e) => {
                    eprintln!("read error: {}", e);
                    self.stream = None;
                    None
                }
            }
        } else {
            None
        }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<(), String> {
        if self.stream.is_none() {
            match self.listener.accept() {
                Ok((stream, _)) => self.stream = Some(stream),
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => return Err("no client connection available".to_string()),
                Err(e) => return Err(format!("failed to accept connection: {}", e)),
            }
        }

        if let Some(ref mut stream) = self.stream {
            stream.write_all(data).map_err(|e| format!("write error: {}", e))?;
            stream.flush().map_err(|e| format!("flush error: {}", e))?;
            Ok(())
        } else {
            Err("no connection established".to_string())
        }
    }
}
