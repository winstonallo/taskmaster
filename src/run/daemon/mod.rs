use std::{collections::HashMap, io::Read, os::unix::net::UnixListener};

use error::DaemonError;

use super::{proc, proc::ProcessError};
use crate::conf;
mod error;
mod monitor;

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
            .map(|(proc_name, proc)| (proc_name.clone(), proc::Process::from_process_config(proc)))
            .collect::<HashMap<String, proc::Process<'tm>>>();

        let client_stream = UnixSocketStream::new(conf.socketpath()).expect("could not create client stream for communication with daemon");

        Self {
            processes: procs,
            client_stream: Box::new(client_stream),
        }
    }

    pub fn get_processes(&self) -> &HashMap<String, proc::Process<'tm>> {
        &self.processes
    }

    fn init(&mut self) -> Result<(), ProcessError> {
        for (proc_name, proc) in &mut self.processes {
            if proc.config().autostart() {
                match proc.start() {
                    Ok(()) => {}
                    Err(err) => {
                        eprintln!("could not start process {proc_name}: {err}");
                    }
                };
            }
        }
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), DaemonError> {
        match self.init() {
            Ok(()) => {}
            Err(err) => {
                eprintln!("could not initialize processes: {err}");
            }
        }
        // Poll for client events and run checks to see if some processes need to be restarted/killed, etc.
        loop {
            if let Some(data) = self.client_stream.poll() {
                println!("received data: {:?}", String::from_utf8(data));
            }
        }
    }
}
