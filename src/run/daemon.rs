use std::{collections::HashMap, error::Error, hash::Hash};

use error::DaemonError;
use libc::stat;
use socket::AsyncUnixSocket;

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
    processes: HashMap<String, proc::Process<'tm>>,
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
}

pub fn monitor_state(mut procs: HashMap<String, Process>) {
    for proc in procs.values_mut() {
        statemachine::monitor_state(proc);
    }
}

pub async fn run<'tm>(socketpath: String, authgroup: String) -> Result<(), Box<dyn Error>> {
    tokio::spawn({
        async move {
            // Removed unused constant declaration causing syntax errors
            let mut client_stream = AsyncUnixSocket::new(&socketpath, &authgroup).unwrap();

            let mut line = String::new();
            loop {
                match client_stream.read_line(&mut line).await {
                    Ok(0) => continue,
                    Ok(_) => {
                        let raw: JsonRPCRaw = match serde_json::from_str(&line) {
                            Ok(raw) => raw,
                            Err(e) => {
                                log_error!("could not parse JSON-RPC: {}", e); // TODO: write error response to socket
                                continue;
                            }
                        };

                        let msg = match JsonRPCMessage::try_from(raw) {
                            Ok(msg) => msg,
                            Err(e) => {
                                log_error!("could not parse JSON-RPC: {:?}", e);
                                continue;
                            }
                        };

                        let msg = match msg {
                            JsonRPCMessage::Request(req) => req,
                            _ => {
                                // server should not receive anything else than requests
                                todo!()
                            }
                        };
                    }
                    Err(e) => {
                        log_error!("{}", e); // TODO: write error response to socket
                        continue;
                    }
                }
            }
        }
    });

    let _ = tokio::signal::ctrl_c().await;

    Ok(())
}
