use std::{collections::HashMap, io::Read, os::unix::net::UnixListener};

use error::DaemonError;

use super::proc;
use crate::conf;
mod error;
mod statemachine;

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
                let mut req = String::new();
                match socket.read_to_string(&mut req) {
                    Ok(val) => {
                        if val > 0 {
                            Some(req.as_bytes().to_vec())
                        } else {
                            None
                        }
                    }
                    Err(err) => {
                        eprintln!("could not read: {err}");
                        None
                    }
                }
            }
            Err(_) => None,
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
            .map(|(proc_name, proc)| (proc_name.clone(), proc::Process::from_process_config(proc, proc_name)))
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
                println!("received data: {:?}", String::from_utf8(data));
            }

            for proc in self.processes.values_mut() {
                statemachine::monitor_state(proc);
            }
        }
    }
}
