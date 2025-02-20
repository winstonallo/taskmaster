use tasklib::conf::Config;
use tasklib::log;
use tasklib::run::daemon::Daemon;

fn main() {
    let conf = match Config::from_file("./config/example.toml") {
        Ok(c) => c,
        Err(err) => {
            eprintln!("could not serialize config: {err}");
            return;
        }
    };

    let mut daemon = Daemon::from_config(&conf);
    log::info(format_args!("starting taskmaster.."));
    let _ = daemon.run();
}
