use crate::log_error;

pub enum CommandError {
    NotFound,
}

pub struct CommandHandler {}

impl CommandHandler {
    fn status() -> Result<String, CommandError> {
        Ok("".to_string())
    }

    fn start() -> Result<String, CommandError> {
        Ok("".to_string())
    }

    fn stop() -> Result<String, CommandError> {
        Ok("".to_string())
    }

    fn restart() -> Result<String, CommandError> {
        Ok("".to_string())
    }

    fn reload_config() -> Result<String, CommandError> {
        Ok("".to_string())
    }

    fn exit() -> Result<String, CommandError> {
        Ok("".to_string())
    }

    fn run(command: &str) -> Result<String, CommandError> {
        match command {
            "status" => CommandHandler::status(),
            "start" => CommandHandler::start(),
            "stop" => CommandHandler::stop(),
            "restart" => CommandHandler::restart(),
            "reload_config" => CommandHandler::reload_config(),
            "exit" => CommandHandler::exit(),
            _ => {
                log_error!("{} does not exist", command);
                Err(CommandError::NotFound)
            }
        }
    }
}
