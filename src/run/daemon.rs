use std::{
    collections::HashMap,
    error::Error,
    hash::Hash,
    io::{Write, stdout},
    time::Duration,
};

use error::DaemonError;
use libc::stat;
use socket::AsyncUnixSocket;
use tokio::time::sleep;

use super::{
    proc::{self, Process},
    statemachine,
};
use crate::{
    conf,
    jsonrpc::{JsonRPCMessage, JsonRPCRaw},
    log_error, log_info,
};
mod command;
mod error;
mod socket;

pub struct Daemon<'tm> {
    pub processes: HashMap<String, proc::Process<'tm>>,
}

impl<'tm> Daemon<'tm> {
    pub fn from_config(conf: &'tm conf::Config) -> Result<Self, Box<dyn Error>> {
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

        Ok(Self { processes: procs })
    }

    pub fn processes_mut(&mut self) -> &HashMap<String, Process> {
        &mut self.processes
    }
}

pub fn monitor_state(procs: &mut HashMap<String, Process>) {
    for proc in procs.values_mut() {
        statemachine::monitor_state(proc);
    }
}

pub async fn run<'tm>(procs: &mut HashMap<String, Process<'_>>, socketpath: String, authgroup: String) -> Result<(), Box<dyn Error>> {
    let mut server_socket = AsyncUnixSocket::new(&socketpath, &authgroup).unwrap();

    loop {
        tokio::select! {
            accept_result = server_socket.accept() => {
                match accept_result {
                    Ok(()) => {
                        let mut socket_for_task = server_socket;
                        tokio::spawn(async move {
                            let mut line = String::new();
                            match socket_for_task.read_line(&mut line).await {
                                Ok(0) => { /* connection closed, do nothing */ },
                                Ok(_) => {
                                    println!("{}", line);
                                },
                                Err(e) => {
                                    log_error!("Error reading from socket: {}", e);
                                }
                            }
                        });
                        server_socket = AsyncUnixSocket::new(&socketpath, &authgroup)?;
                    },
                    Err(e) => {
                        log_error!("Failed to accept connection: {}", e);
                    }
                }
            },
            _ = sleep(Duration::from_millis(1)) => {
                monitor_state(procs);
            }
        }
    }
}
