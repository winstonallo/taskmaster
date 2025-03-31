#![allow(unused)]

use std::fmt::{Debug, format};

use crate::{jsonrpc::JsonRPCRequest, log_error, log_info};

use super::Daemon;

pub enum CommandError {
    NotFound,
}

impl Daemon {
    fn status(&self, args: Vec<&str>) -> Result<String, CommandError> {
        let mut response: Vec<String> = vec![];

        if args.len() < 2 {
            for (proc_name, proc) in &self.processes {
                response.push(format!("{}: {}", proc_name, proc.state()));
            }
            return Ok(response.join("\n"));
        }

        for arg in &args[1..] {
            for (proc_name, proc) in &self.processes {
                if let Some(pid) = proc.id() {
                    if let Ok(id) = arg.parse::<u32>() {
                        if id == pid {
                            response.push(format!("{}: {}", proc_name, proc.state()));
                            break;
                        }
                    }
                }
                let base_name: &str = proc_name.split("_").collect::<Vec<&str>>()[0];
                if arg == proc_name || arg == &base_name {
                    response.push(format!("{}: {}", proc_name, proc.state()));
                    break;
                }
            }
        }

        Ok(response.join("\n"))
    }

    fn start(&self, args: Vec<&str>) -> Result<String, CommandError> {
        Ok("".to_string())
    }

    fn stop(&self, args: Vec<&str>) -> Result<String, CommandError> {
        Ok("".to_string())
    }

    fn restart(&self, args: Vec<&str>) -> Result<String, CommandError> {
        Ok("".to_string())
    }

    fn reload_config(&self, args: Vec<&str>) -> Result<String, CommandError> {
        Ok("".to_string())
    }

    fn exit(&self, args: Vec<&str>) -> Result<String, CommandError> {
        Ok("".to_string())
    }

    pub fn run_command(&self, request: &JsonRPCRequest) -> Result<String, CommandError> {
        log_info!("got command: {:?}", request);
        let command: &str = &request.method;
        // match command[0] {
        //     "status" => self.status(command),
        //     "start" => self.start(command),
        //     "stop" => self.stop(command),
        //     "restart" => self.restart(command),
        //     "reload-config" => self.reload_config(command),
        //     "exit" => self.exit(command),
        //     _ => {
        //         log_error!("{} does not exist", command[0]);
        //         Err(CommandError::NotFound)
        //     }
        // }
        Ok("".to_string())
    }
}
