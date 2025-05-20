use std::{
    env::{self},
    error::Error,
    fs::remove_file,
};

use tasklib::{conf::Config, log, log_info, run::daemon::Daemon};

pub const PID_FILE_PATH: &str = "/tmp/taskmaster.pid";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    #[cfg(not(unix))]
    {
        panic!("taskmaster only support UNIX systems");
    }

    if env::args().len() != 2 {
        eprintln!("usage: ./taskmaster ./path/to/config.toml");
        std::process::exit(1);
    }

    let arguments: Vec<String> = env::args().collect();

    let arg: String = arguments.get(1).unwrap().to_owned();

    let conf = match Config::from_file(&arg) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1)
        }
    };

    log::init(conf.logfile())?;

    let mut daemon = Daemon::from_config(conf, arg);

    log_info!("starting taskmaster..");

    daemon.run().await?;

    let _ = remove_file(PID_FILE_PATH);

    Ok(())
}
