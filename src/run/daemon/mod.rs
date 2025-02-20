use std::{
    collections::HashMap,
    io::{ErrorKind, Read},
    os::unix::net::UnixListener,
};

use error::DaemonError;

use super::{proc, statemachine};
use crate::{conf, log_error};
mod error;

trait ClientStream {
    fn poll(&self) -> Option<Vec<u8>>;
}

#[allow(unused)]
struct UnixSocketStream {
    path: String,
    listener: UnixListener,
}

impl UnixSocketStream {
    fn new(path: &str) -> Result<Self, String> {
        let _ = std::fs::remove_file(path);
        let listener = UnixListener::bind(path).map_err(|err| format!("could not bind to socket at path: {path}: {err}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|err| format!("failed to set non-blocking mode: {err}"))?;
        Ok(Self {
            path: path.to_string(),
            listener,
        })
    }
}

impl ClientStream for UnixSocketStream {
    fn poll(&self) -> Option<Vec<u8>> {
        match self.listener.accept() {
            Ok((mut socket, addr)) => {
                println!("got client: {:?} - {:?}", socket, addr);
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

pub struct Daemon<'tm> {
    processes: HashMap<String, proc::Process<'tm>>,
    client_stream: Box<dyn ClientStream>,
}

impl<'tm> Daemon<'tm> {
    pub fn from_config(conf: &'tm conf::Config) -> Self {
        let procs: HashMap<String, proc::Process<'tm>> = conf
            .processes()
            .iter()
            .flat_map(|(proc_name, proc)| {
                (0..proc.processes()).map(move |id| {
                    let key = if proc.processes() > 1 {
                        format!("{}_{}", proc_name, id)
                    } else {
                        proc_name.to_owned()
                    };
                    (key.clone(), proc::Process::from_process_config(proc, &key))
                })
            })
            .collect::<HashMap<String, proc::Process<'tm>>>();

        let client_stream = UnixSocketStream::new(conf.socketpath()).expect("could not create client stream for communication with daemon");

        Self {
            processes: procs,
            client_stream: Box::new(client_stream),
        }
    }

    pub fn run(&mut self) -> Result<(), DaemonError> {
        loop {
            if let Some(data) = self.client_stream.poll() {
                log_error!("received data: {:?}", String::from_utf8(data));
            }

            for proc in self.processes.values_mut() {
                statemachine::monitor_state(proc);
            }
        }
    }
}
