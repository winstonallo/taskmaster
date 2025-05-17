use std::env;

use crate::{conf::defaults::dflt_socketpath, jsonrpc::request::AttachFile};

pub fn help() -> String {
    let mut help_text = String::new();

    help_text.push_str("USAGE:\n");
    help_text.push_str("  taskshell [OPTIONS] COMMAND [ARGS]\n\n");

    help_text.push_str("OPTIONS:\n");
    help_text.push_str("  -s, --socketpath PATH      Path to taskmaster socket [default: $TASKMASTER_SOCKETPATH or /tmp/taskmaster.sock]\n\n");

    help_text.push_str("COMMANDS:\n");
    help_text.push_str("  status [PROCESS]           Show status of all processes or a specific process\n");
    help_text.push_str("  start PROCESS              Start a process\n");
    help_text.push_str("  restart PROCESS            Restart a process\n");
    help_text.push_str("  stop PROCESS               Stop a process\n");
    help_text.push_str("  attach PROCESS SUBCOMMAND  Attach to process output\n");
    help_text.push_str("  reload                     Reload the configuration\n");
    help_text.push_str("  exit                       Exit the shell\n");
    help_text.push_str("  engine SUBCOMMAND          Control the taskmaster engine\n");
    help_text.push_str("    start CONFIG_PATH        Start the taskmaster engine with the given configuration\n");
    help_text.push_str("    stop                     Stop the taskmaster engine\n");

    help_text
}

#[derive(Debug, PartialEq)]
pub enum EngineSubcommand {
    Start { config_path: String },
    Stop,
}

impl TryFrom<Vec<String>> for EngineSubcommand {
    type Error = String;

    fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
        match value[0].as_str() {
            "start" => {
                if value.len() != 2 {
                    return Err("engine start CONFIG_PATH".to_string());
                }
                Ok(Self::Start {
                    config_path: value[1].to_owned(),
                })
            }
            "stop" => {
                if value.len() != 1 {
                    return Err("no argument expected for 'engine stop'".to_string());
                }
                Ok(Self::Stop)
            }
            _ => Err(format!("{value:?}: invalid subcommand for 'engine' (expected start | stop)")),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ShellCommand {
    Status { process: Option<String> },
    Start { process: String },
    Restart { process: String },
    Stop { process: String },
    Attach { process: String, fd: AttachFile },
    Reload,
    Exit,
    Engine { subcommand: EngineSubcommand },
    Help,
}

impl TryFrom<Vec<String>> for ShellCommand {
    type Error = String;

    fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err("".to_string());
        }
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
                let subcommand = EngineSubcommand::try_from(value[1..].to_vec())?;
                Ok(Self::Engine { subcommand })
            }
            "help" => Ok(Self::Help),
            _ => Err("command not found".to_string()),
        }
    }
}

#[derive(Debug)]
pub struct Args {
    command: ShellCommand,
    socketpath: String,
}

impl Args {
    pub fn command(&self) -> &ShellCommand {
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
            if arg.as_str() == "-s" || arg.as_str() == "--socketpath" {
                if value.len() <= idx + 1 {
                    return Err(format!("--socketpath option expected a value\n\n{}", help()));
                }
                socketpath = Some(value[idx + 1].to_owned());
                value.remove(idx);
                value.remove(idx);
                break;
            }
        }

        if socketpath.is_none() {
            socketpath = Some(env::var("TASKMASTER_SOCKETPATH").unwrap_or(dflt_socketpath()));
        }
        let command = match ShellCommand::try_from(value) {
            Ok(command) => command,
            Err(e) => return Err(format!("{e}\n\n{}", help())),
        };
        Ok(Self {
            command,
            socketpath: socketpath.unwrap(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    static ENV_LOCK: std::sync::Mutex<bool> = Mutex::new(true);

    #[test]
    fn full_command_line() {
        let command_line = "engine stop --socketpath taskmaster.sock"
            .to_string()
            .split_ascii_whitespace()
            .map(String::from)
            .collect::<Vec<String>>();

        let args = Args::try_from(command_line).unwrap();

        assert_eq!(
            args.command,
            ShellCommand::Engine {
                subcommand: EngineSubcommand::Stop
            }
        );
        assert_eq!(args.socketpath, "taskmaster.sock".to_string());
    }

    #[test]
    fn default_socketpath() {
        let _handle = ENV_LOCK.lock();
        let command_line = "engine stop"
            .to_string()
            .split_ascii_whitespace()
            .map(String::from)
            .collect::<Vec<String>>();

        let args = Args::try_from(command_line).unwrap();

        assert_eq!(args.socketpath, "/tmp/taskmaster.sock".to_string());
    }

    #[test]
    fn socketpath_from_env() {
        let _handle = ENV_LOCK.lock();
        unsafe {
            std::env::set_var("TASKMASTER_SOCKETPATH", "taskmaster.sock");
        }

        let command_line = "engine stop"
            .to_string()
            .split_ascii_whitespace()
            .map(String::from)
            .collect::<Vec<String>>();

        let args = Args::try_from(command_line).unwrap();

        assert_eq!(args.socketpath, "taskmaster.sock".to_string());

        unsafe {
            std::env::remove_var("TASKMASTER_SOCKETPATH");
        }
    }

    #[test]
    fn cli_arg_overrides_env() {
        let _handle = ENV_LOCK.lock();
        unsafe {
            std::env::set_var("TASKMASTER_SOCKETPATH", "taskmaster.sock");
        }

        let command_line = "engine stop --socketpath /tmp/taskmaster.sock"
            .to_string()
            .split_ascii_whitespace()
            .map(String::from)
            .collect::<Vec<String>>();

        let args = Args::try_from(command_line).unwrap();

        assert_eq!(args.socketpath, "/tmp/taskmaster.sock".to_string());

        unsafe {
            std::env::remove_var("TASKMASTER_SOCKETPATH");
        }
    }

    #[test]
    fn unknown_command() {
        let command_line = "not a command"
            .to_string()
            .split_ascii_whitespace()
            .map(String::from)
            .collect::<Vec<String>>();

        assert!(Args::try_from(command_line).is_err());
    }

    #[test]
    fn unknown_engine_subcommand() {
        let command_line = "engine dance"
            .to_string()
            .split_ascii_whitespace()
            .map(String::from)
            .collect::<Vec<String>>();

        assert!(Args::try_from(command_line).is_err());
    }

    #[test]
    fn socketpath_option_without_arg() {
        let command_line = "status --socketpath"
            .to_string()
            .split_ascii_whitespace()
            .map(String::from)
            .collect::<Vec<String>>();

        assert!(Args::try_from(command_line).is_err());
    }

    #[test]
    fn unknown_attach_subcommand() {
        let command_line = "attach foo painting"
            .to_string()
            .split_ascii_whitespace()
            .map(String::from)
            .collect::<Vec<String>>();

        assert!(Args::try_from(command_line).is_err());
    }
}
