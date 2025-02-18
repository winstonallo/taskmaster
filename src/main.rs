use conf::Config;
use run::daemon::Daemon;

mod conf;
mod log;
mod run;

fn main() {
    let conf = match Config::from_file("./config/example.toml") {
        Ok(c) => c,
        Err(err) => {
            eprintln!("could not serialize config: {err}");
            return;
        }
    };

    let mut daemon = Daemon::from_config(&conf);
    println!("{:?}", daemon.get_processes());
    let _ = daemon.run();
}
