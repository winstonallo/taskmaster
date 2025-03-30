use std::{collections::HashMap, error::Error, sync::Arc, time::Duration};

use socket::AsyncUnixSocket;
use tokio::time::sleep;

use super::{
    proc::{self, Process},
    statemachine::{self},
};

use crate::{
    conf,
    jsonrpc::{self, JsonRPCRequest},
    log_error, log_info,
};
mod command;
mod error;
mod socket;

pub struct Daemon {
    pub processes: HashMap<String, proc::Process>,
}

impl Daemon {
    pub fn from_config(conf: &conf::Config) -> Result<Self, Box<dyn Error>> {
        let procs: HashMap<String, proc::Process> = conf
            .processes()
            .iter()
            .flat_map(|(proc_name, proc)| {
                (0..proc.processes()).map(move |id| {
                    let key = if proc.processes() > 1 {
                        format!("{}_{}", proc_name, id)
                    } else {
                        proc_name.to_owned()
                    };
                    (key.clone(), proc::Process::from_process_config(proc.clone(), &key))
                })
            })
            .collect::<HashMap<String, proc::Process>>();

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

async fn handle_client(mut socket: AsyncUnixSocket, sender: Arc<tokio::sync::mpsc::Sender<(JsonRPCRequest, AsyncUnixSocket)>>) {
    let mut line = String::new();
    match socket.read_line(&mut line).await {
        Ok(0) => { /* connection closed, do nothing */ }
        Ok(_) => {
            match serde_json::from_str(&line) {
                Ok(request) => {
                    let _ = sender.send((request, socket)).await;
                }
                Err(e) => {
                    log_error!("error deserializing request: {}", e)
                }
            }

            // if let Err(e) = socket.write(line.as_bytes()).await {
            //     log_error!("error writing to client: {}", e);
            // }
        }
        Err(e) => {
            log_error!("Error reading from socket: {}", e);
        }
    }
}

pub async fn run(procs: &mut HashMap<String, Process>, socketpath: String, authgroup: String) -> Result<(), Box<dyn Error>> {
    let mut listener = AsyncUnixSocket::new(&socketpath, &authgroup).unwrap();

    let (sender, mut reciever) = tokio::sync::mpsc::channel(1024);
    let sender = Arc::new(sender);

    loop {
        tokio::select! {
            accept_result = listener.accept() => {

                if let Err(e) = accept_result {
                    log_error!("Failed to accept connection: {}", e);
                    continue;
                }

                let socket = listener;
                let clone = sender.clone();
                tokio::spawn(async move {
                    handle_client(socket, clone).await;
                });

                listener = AsyncUnixSocket::new(&socketpath, &authgroup)?;
            },
            Some((request, mut socket)) = reciever.recv() => {
                if let Some(resp) = jsonrpc::handle_halt(&request) {
                        match serde_json::to_string(&resp) {
                            Err(_) => {},
                            Ok(s) => {
                                tokio::spawn(async move {
                                   let _ = socket.write(s.as_bytes()).await;
                                });
                            }
                        }

                    for p in procs.iter_mut() {
                        let _ = p.1.stop();
                    }
                    log_info!("shutting down taskmaster...");
                    return Ok(());
                }

                let res = jsonrpc::handle(request, procs);

                match res {
                    Ok(resp) => {
                        match serde_json::to_string(&resp) {
                            Err(_) => {},
                            Ok(s) => {
                                tokio::spawn(async move {
                                   let _ = socket.write(s.as_bytes()).await;
                                });
                            }
                        }
                    },
                    Err(err) => {
                        match serde_json::to_string(&err) {
                            Err(_) => {},
                            Ok(s) => {
                                tokio::spawn(async move {
                                   let _ = socket.write(s.as_bytes()).await;
                                });
                            }
                        }
                    }
                };
             },
            _ = sleep(Duration::from_nanos(1)) => {
                monitor_state(procs);
            }
        }
    }
}
