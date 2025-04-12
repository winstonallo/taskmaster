use std::{collections::HashMap, error::Error, sync::Arc, time::Duration};

use socket::AsyncUnixSocket;
use tokio::time::sleep;

use super::proc::{self, Process};

use crate::{
    conf,
    jsonrpc::{handlers::handle_request, request::Request},
    log_error,
};
mod error;
mod socket;

pub struct Daemon {
    processes: HashMap<String, proc::Process>,
    socket_path: String,
    auth_group: String,
    config_path: String,
}

impl Daemon {
    pub fn try_from_config(conf: conf::Config, config_path: String) -> Result<Self, Box<dyn Error>> {
        let processes: HashMap<String, proc::Process> = conf
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

        Ok(Self {
            processes,
            socket_path: conf.socketpath().to_owned(),
            auth_group: conf.authgroup().to_owned(),
            config_path,
        })
    }
    pub fn processes(&mut self) -> &HashMap<String, Process> {
        &self.processes
    }

    pub fn processes_mut(&mut self) -> &mut HashMap<String, Process> {
        &mut self.processes
    }

    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }

    pub fn auth_group(&self) -> &str {
        &self.auth_group
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let mut listener = AsyncUnixSocket::new(self.socket_path(), self.auth_group()).unwrap();

        let (sender, mut receiver) = tokio::sync::mpsc::channel(1024);
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

                    listener = AsyncUnixSocket::new(self.socket_path(), self.auth_group())?;
                },

                Some((request, mut socket)) = receiver.recv() => {
                    let response = handle_request(self, request);

                    let msg = serde_json::to_string(&response).unwrap();

                    tokio::spawn(async move {
                        if let Err(e) = socket.write(msg.as_bytes()).await {
                            log_error!("error sending to socket: {}", e);
                        }
                    });
                },

                _ = sleep(Duration::from_nanos(1)) => {
                    monitor_state(self.processes_mut());
                }
            }
        }
    }
}

pub fn monitor_state(procs: &mut HashMap<String, Process>) {
    for proc in procs.values_mut() {
        proc.desire();
        proc.monitor();
    }
}

async fn handle_client(mut socket: AsyncUnixSocket, sender: Arc<tokio::sync::mpsc::Sender<(Request, AsyncUnixSocket)>>) {
    let mut line = String::new();

    match socket.read_line(&mut line).await {
        Ok(0) => { /* connection closed, do nothing */ }
        Ok(_) => match serde_json::from_str(&line) {
            Ok(request) => {
                let _ = sender.send((request, socket)).await;
            }
            Err(e) => {
                if let Err(e) = socket.write(format!("{}", e).as_bytes()).await {
                    log_error!("error writing to socket: {}", e)
                }
            }
        },
        Err(e) => {
            log_error!("Error reading from socket: {}", e);
        }
    }
}
