use run::daemon::Daemon;

mod conf;
mod run;

fn main() {
    let conf = match conf::Config::from_file("./config/example.toml") {
        Ok(c) => c,
        Err(err) => {
            eprintln!("could not serialize config: {err}");
            return;
        }
    };
    println!("{:?}", conf.get_processes()["nginx"]);

    let daemon = Daemon::from_config(&conf);
    println!("{:?}", daemon.get_processes()["nginx"]);
}
