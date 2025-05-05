use std::env;

use crate::{conf::defaults::dflt_socketpath, jsonrpc::request::AttachFile};

pub enum EngineSubcommand {
    Start,
    Stop,
}

impl TryFrom<&str> for EngineSubcommand {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "start" => Ok(Self::Start),
            "stop" => Ok(Self::Stop),
            _ => Err(format!("{value}: invalid subcommand for 'engine' (expected start | stop)")),
        }
    }
}

pub enum Command {
    Status { process: Option<String> },
    Start { process: String },
    Restart { process: String },
    Stop { process: String },
    Attach { process: String, fd: AttachFile },
    Reload,
    Exit,
    Engine { subcommand: EngineSubcommand },
}

impl TryFrom<Vec<String>> for Command {
    type Error = String;

    fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
        match value[0].as_str() {
            "status" => {
                if value.len() > 2 {
                    return Err("status [PROCESS_NAME]".to_string());
                }
                Ok(Self::Status {
                    process: value.get(1).map(String::to_owned),
                })
            }
            "start" => {
                if value.len() != 2 {
                    return Err("start PROCESS_NAME".to_string());
                }
                Ok(Self::Start { process: value[1].to_owned() })
            }
            "restart" => {
                if value.len() != 2 {
                    return Err("restart PROCESS_NAME".to_string());
                }
                Ok(Self::Restart { process: value[1].to_owned() })
            }
            "stop" => {
                if value.len() != 2 {
                    return Err("stop PROCESS_NAME".to_string());
                }
                Ok(Self::Stop { process: value[1].to_owned() })
            }
            "attach" => {
                if value.len() != 3 {
                    return Err("attach PROCESS_NAME {stdout | stderr}".to_string());
                }
                let fd = AttachFile::try_from(value[2].as_str())?;
                Ok(Self::Attach {
                    process: value[1].to_owned(),
                    fd,
                })
            }
            "reload" => {
                if value.len() != 1 {
                    return Err("reload".to_string());
                }
                Ok(Self::Reload)
            }
            "exit" => {
                if value.len() != 1 {
                    return Err("exit".to_string());
                }
                Ok(Self::Exit)
            }
            "engine" => {
                if value.len() != 2 {
                    return Err("engine {start | stop}".to_string());
                }
                let subcommand = EngineSubcommand::try_from(value[1].as_str())?;
                Ok(Self::Engine { subcommand })
            }
            _ => {
                // Return help message
                Err(String::new())
            }
        }
    }
}

pub struct Args {
    command: Command,
    socketpath: String,
}

impl Args {
    pub fn command(&self) -> &Command {
        &self.command
    }

    pub fn socketpath(&self) -> &str {
        &self.socketpath
    }
}

impl TryFrom<Vec<String>> for Args {
    type Error = String;

    fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
        let mut value = value.clone();
        let mut socketpath: Option<String> = None;

        for (idx, arg) in value.iter().enumerate() {
            if (arg.as_str() == "-s" || arg.as_str() == "--socketpath") && value.len() > idx + 1 {
                socketpath = Some(value[idx + 1].to_owned());
                value.remove(idx);
                value.remove(idx + 1);
                break;
            }
        }

        if socketpath.is_none() {
            socketpath = match env::var("TASKMASTER_SOCKETPATH") {
                Ok(val) => Some(val),
                Err(_) => Some(dflt_socketpath()),
            }
        }
        let command = Command::try_from(value)?;
        Ok(Self {
            command,
            socketpath: socketpath.unwrap(),
        })
    }
}
