use std::{collections::HashMap, io::Read, os::unix::net::UnixListener};

use super::proc;
use crate::conf;

trait ClientStream {
    fn poll(&self) -> Option<Vec<u8>>;
}

struct UnixSocketStream {
    path: String,
    listener: UnixListener,
}

impl UnixSocketStream {
    fn new(path: &str) -> Result<Self, String> {
        let _ = std::fs::remove_file(path);
        let listener = UnixListener::bind(path).map_err(|err| format!("could not bind to socket at path: {path}: {err}"))?;
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
            Err(err) => {
                eprintln!("could not accept: {err}");
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
            .get_processes()
            .iter()
            .map(|(proc_name, proc)| (proc_name.clone(), proc::Process::from_process_config(proc)))
            .collect::<HashMap<String, proc::Process<'tm>>>();

        let client_stream = UnixSocketStream::new("/tmp/.taskmaster.sock").expect("could not create client stream for communication with daemon");

        Self {
            processes: procs,
            client_stream: Box::new(client_stream),
        }
    }

    pub fn get_processes(&self) -> &HashMap<String, proc::Process<'tm>> {
        &self.processes
    }

    pub fn poll(&self) {
        if let Some(data) = self.client_stream.poll() {
            println!("received data: {:?}", data)
        }
    }
}
