use std::{collections::HashMap, error::Error, sync::Arc, time::Duration};

use serde::{Deserialize, Serialize};
use socket::AsyncUnixSocket;
use tokio::time::sleep;

use super::proc::{self, Process};
use super::statemachine::states::ProcessState;
use crate::jsonrpc::response::{Response, ResponseError, ResponseType};
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
    shutting_down: bool,
}

impl Daemon {
    pub fn from_config(conf: conf::Config, config_path: String) -> Self {
        let processes: HashMap<String, proc::Process> = conf
            .processes()
            .iter()
            .flat_map(|(proc_name, proc)| {
                (0..proc.processes()).map(move |id| {
                    let key = if proc.processes() > 1 {
                        format!("{proc_name}_{id}")
                    } else {
                        proc_name.to_owned()
                    };
                    (key.clone(), proc::Process::from_process_config(proc.clone(), &key))
                })
            })
            .collect::<HashMap<String, proc::Process>>();

        Self {
            processes,
            socket_path: conf.socketpath().to_owned(),
            auth_group: conf.authgroup().to_owned(),
            config_path,
            shutting_down: false,
        }
    }

    pub fn processes(&self) -> &HashMap<String, Process> {
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

    pub fn config_path(&self) -> &str {
        &self.config_path
    }

    pub fn shutting_down(&self) -> bool {
        self.shutting_down
    }

    pub fn shutdown(&mut self) {
        self.shutting_down = true;
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

                    let mut socket = listener;
                    let clone = sender.clone();

                    let shutting_down = self.shutting_down;
                    tokio::spawn(async move {
                        if shutting_down {

                            let _ = socket.write("not accepting requests - currently shutting down".as_bytes()).await;
                        } else {
                            handle_client(socket, clone).await;
                        }
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

                    if  self.shutting_down && self.no_process_running(){
                        return Ok(());
                    }
                }
            }
        }
    }

    pub fn no_process_running(&self) -> bool {
        let mut no_process_running = true;
        for proc in self.processes().values() {
            use ProcessState::*;
            match proc.state() {
                Ready | HealthCheck(_) | Healthy | Stopping(_) => no_process_running = false,
                _ => {}
            }
        }
        no_process_running
    }
}

fn monitor_state(procs: &mut HashMap<String, Process>) {
    for proc in procs.values_mut() {
        proc.desire();
        proc.monitor();
    }
}

#[derive(Serialize, Deserialize)]
pub struct MininamRequest {
    pub id: u32,
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
                let error_msg = match serde_json::from_str::<MininamRequest>(&line) {
                    Ok(m_r) => serde_json::to_string(&Response::new(
                        m_r.id,
                        ResponseType::Error(ResponseError {
                            code: crate::jsonrpc::response::ErrorCode::InvalidRequest,
                            message: format!("{e}").to_owned(),
                            data: None,
                        }),
                    ))
                    .unwrap(),
                    Err(_) => "request id not found - can't respond with JsonRPCError".to_owned(),
                };
                if let Err(e) = socket.write(error_msg.as_bytes()).await {
                    log_error!("error writing to socket: {}", e)
                }
            }
        },
        Err(e) => {
            log_error!("Error reading from socket: {}", e);
        }
    }
}
