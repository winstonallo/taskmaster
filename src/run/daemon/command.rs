#![allow(unused)]

use crate::{log_error, log_info};

use super::Daemon;

pub enum CommandError {
    NotFound,
}

impl Daemon<'_> {
    fn status(&self, args: Vec<&str>) -> Result<String, CommandError> {
        let mut response: Vec<String> = vec![];

        if args.len() < 2 {
            // get status for all processes
            // for arg in args {
            for (proc_name, proc) in &self.processes {
                response.push(format!("{:?}", proc.state()));
                // if let Ok(id) = proc_name.parse::<u32>() {
                //     if let Some(pid) = proc.id() {
                //         if id == pid {
                //             break;
                //         }
                //     }
                // }
                // if arg == proc_name {
                //     response.push(format!("{:?}", proc.state()));
                // }
                // // }
            }
            return Ok(response.join("\n"));
        }

        for arg in &args[1..] {
            // if arg == some pid or arg == some process name { append status }
            // else { append error }
        }

        Ok("".to_string())
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

    pub fn run_command(&self, input: &str) -> Result<String, CommandError> {
        log_info!("got command: {}", input);
        let command: Vec<&str> = input.split_whitespace().collect();
        match command[0] {
            "status" => self.status(command),
            "start" => self.start(command),
            "stop" => self.stop(command),
            "restart" => self.restart(command),
            "reload-config" => self.reload_config(command),
            "exit" => self.exit(command),
            _ => {
                log_error!("{} does not exist", command[0]);
                Err(CommandError::NotFound)
            }
        }
    }
}
