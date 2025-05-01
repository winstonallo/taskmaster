use std::{
    env::{self},
    error::Error,
    fs::remove_file,
    io::Write,
};

use tasklib::{conf::Config, log, log_error, log_info, run::daemon::Daemon};

const PID_FILE_PATH: &'static str = "/tmp/taskmaster.pid";

fn write_pid_file() -> Result<(), Box<dyn Error>> {
    let pid = unsafe { libc::getpid() };
    let mut pid_file = std::fs::File::create(PID_FILE_PATH)?;
    pid_file.write(pid.to_string().as_bytes())?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if env::args().len() != 2 {
        log_error!("usage: ./taskmaster ./path/to/config.toml");
        std::process::exit(1);
    }

    let arguments: Vec<String> = env::args().collect();

    let arg: String = arguments.get(1).unwrap().to_owned();

    let conf = match Config::from_file(&arg) {
        Ok(c) => c,
        Err(e) => {
            log_error!("{e}");
            std::process::exit(1)
        }
    };

    write_pid_file()?;

    log::init(conf.logfile())?;

    let mut daemon = Daemon::from_config(conf, arg);

    log_info!("starting taskmaster..");

    daemon.run().await?;

    let _ = remove_file(PID_FILE_PATH);

    Ok(())
}
