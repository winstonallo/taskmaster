use std::error::Error;

use tasklib::conf::Config;
use tasklib::run::daemon::Daemon;
use tasklib::{log, log_error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let conf = match Config::from_file("./config/example.toml") {
        Ok(c) => c,
        Err(e) => {
            log_error!("{}", e);
            return Err(e);
        }
    };

    let mut daemon = match Daemon::from_config(&conf) {
        Ok(d) => d,
        Err(e) => {
            log_error!("{}", e);
            return Err(e);
        }
    };
    log::info(format_args!("starting taskmaster.."));
    let _ = daemon.run(&conf).await;

    tokio::signal::ctrl_c().await?;

    Ok(())
}
