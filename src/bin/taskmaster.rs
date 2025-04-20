use std::{
    env::{self},
    error::Error,
};

use tasklib::{conf::Config, log_error, log_info, run::daemon::Daemon};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if env::args().len() != 2 {
        log_error!("usage: ./taskmaster ./path/to/config.toml");
        std::process::exit(1);
    }

    let arguments: Vec<String> = env::args().collect();

    let config_path: String = arguments.get(1).unwrap().to_owned();

    let conf = match Config::from_file(&config_path) {
        Ok(c) => c,
        Err(e) => {
            log_error!("{}", e);
            std::process::exit(1)
        }
    };

    let mut daemon = Daemon::from_config(conf, config_path);

    log_info!("starting taskmaster..");
    daemon.run().await
}
