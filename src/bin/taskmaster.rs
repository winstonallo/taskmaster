use std::{
    env::{self, args},
    error::Error,
};

use tasklib::{
    conf::{self, Config},
    log_error, log_info,
    run::daemon::Daemon,
};

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
    // let res = tasklib::run::daemon::run(&mut daemon.processes, conf.socketpath().to_string(), conf.authgroup().to_string()).await;
    // if let Ok(()) = res {
    //     return Ok(());
    // }
    // tokio::signal::ctrl_c().await?;
}
