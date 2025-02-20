use std::collections::HashMap;

use error::DaemonError;
use socket::UnixSocket;

use super::{proc, statemachine};
use crate::conf;
mod command;
mod error;
mod socket;

pub struct Daemon<'tm> {
    processes: HashMap<String, proc::Process<'tm>>,
    client_stream: Box<UnixSocket>,
}

impl<'tm> Daemon<'tm> {
    pub fn from_config(conf: &'tm conf::Config) -> Result<Self, String> {
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

        let client_stream = UnixSocket::new(conf.socketpath(), conf.authgroup()).map_err(|e| format!("unix socket stream creation failed: {}", e))?;

        Ok(Self {
            processes: procs,
            client_stream: Box::new(client_stream),
        })
    }

    pub fn run(&mut self) -> Result<(), DaemonError> {
        loop {
            if let Some(data) = self.client_stream.poll() {
                let data = unsafe { String::from_utf8_unchecked(data) };

                match self.run_command(&data) {
                    Ok(response) => println!("{}", response), // write response to socket
                    Err(_) => {}                              // write error response to socket
                };
            }

            for proc in self.processes.values_mut() {
                statemachine::monitor_state(proc);
            }
        }
    }
}
