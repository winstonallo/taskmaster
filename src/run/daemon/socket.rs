use std::{
    ffi::CString,
    fs::{self},
    io::{ErrorKind, Read},
    os::unix::{fs::PermissionsExt, net::UnixListener},
};

use libc::{chown, getgrnam, gid_t};

#[allow(unused)]
pub struct UnixSocket {
    path: String,
    authgroup: String,
    listener: UnixListener,
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
        if std::fs::metadata(path).is_ok() {
            let _ = std::fs::remove_file(path);
        }
        let listener = UnixListener::bind(path).map_err(|err| format!("could not bind to socket at path: {path}: {err}"))?;

        listener
            .set_nonblocking(true)
            .map_err(|err| format!("failed to set non-blocking mode: {err}"))?;

        let gid = get_group_id(authgroup)?;

        let c_path = CString::new(path).map_err(|e| format!("invalid path: {}", e))?;

        unsafe {
            if chown(c_path.as_ptr(), gid as gid_t, gid as gid_t) != 0 {
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
        })
    }

    pub fn poll(&self) -> Option<Vec<u8>> {
        match self.listener.accept() {
            Ok((mut socket, _)) => {
                let _ = socket.set_nonblocking(true);
                let mut req = String::new();
                match socket.read_to_string(&mut req) {
                    Ok(n) if n > 0 => Some(req.into_bytes()),
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => None,
                    Err(e) => {
                        eprintln!("read error: {e}");
                        None
                    }
                    _ => None,
                }
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => None,
            Err(e) => {
                eprintln!("accept error: {e}");
                None
            }
        }
    }
}
