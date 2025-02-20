use tasklib::conf::Config;
use tasklib::run::daemon::Daemon;
use tasklib::{log, log_error};

fn main() {
    let conf = match Config::from_file("./config/example.toml") {
        Ok(c) => c,
        Err(err) => {
            eprintln!("could not serialize config: {err}");
            return;
        }
    };

    let mut daemon = match Daemon::from_config(&conf) {
        Ok(d) => d,
        Err(e) => {
            log_error!("{}", e);
            return;
        }
    };
    log::info(format_args!("starting taskmaster.."));
    let _ = daemon.run();
}
