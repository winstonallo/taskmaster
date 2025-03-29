use std::{collections::HashMap, error::Error, sync::Arc, time::Duration};

use socket::AsyncUnixSocket;
use tokio::time::sleep;

use super::{
    proc::{self, Process},
    statemachine,
};

use crate::{conf, jsonrpc::JsonRPCRequest, log_error};
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

async fn handle_client(mut socket: AsyncUnixSocket, mut sender: Arc<tokio::sync::mpsc::Sender<JsonRPCRequest>>) {
    let mut line = String::new();
    match socket.read_line(&mut line).await {
        Ok(0) => { /* connection closed, do nothing */ }
        Ok(_) => {
            //println!("{}", line);

            let request = serde_json::from_str(&line).unwrap();

            sender.send(request).await;

            if let Err(e) = socket.write(line.as_bytes()).await {
                log_error!("error writing to client: {}", e);
            }
        }
        Err(e) => {
            log_error!("Error reading from socket: {}", e);
        }
    }
}

pub async fn run(procs: &mut HashMap<String, Process<'_>>, socketpath: String, authgroup: String) -> Result<(), Box<dyn Error>> {
    let mut listener = AsyncUnixSocket::new(&socketpath, &authgroup).unwrap();

    let (sender, mut reciever) = tokio::sync::mpsc::channel(1024);
    let a = Arc::new(sender);
    loop {
        tokio::select! {
            accept_result = listener.accept() => {

                if let Err(e) = accept_result {
                    log_error!("Failed to accept connection: {}", e);
                    continue;
                }

                let socket = listener;
                let temp = a.clone();
                tokio::spawn(async move {
                    handle_client(socket, temp).await;
                });

                listener = AsyncUnixSocket::new(&socketpath, &authgroup)?;
            },
            Some(request) = reciever.recv() => {
                println!("{:?}", request)
            },
            _ = sleep(Duration::from_nanos(1)) => {
                monitor_state(procs);
            }
        }
    }
}
