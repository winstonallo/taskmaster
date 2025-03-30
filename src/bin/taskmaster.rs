use std::error::Error;

use tasklib::conf::Config;
use tasklib::run::daemon::Daemon;
use tasklib::{log_error, log_info};

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

    log_info!("starting taskmaster..");
    let res = tasklib::run::daemon::run(&mut daemon.processes, conf.socketpath().to_string(), conf.authgroup().to_string()).await;
    if let Ok(()) = res {
        return Ok(());
    }
    tokio::signal::ctrl_c().await?;

    Ok(())
}
