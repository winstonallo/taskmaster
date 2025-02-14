use run::daemon::Daemon;

mod conf;
mod run;

fn main() {
    let conf = conf::Config::from_file("./config/example.toml").expect("config construction failed");
    println!("{:?}", conf.get_processes()["nginx"]);

    let daemon = Daemon::from_config(&conf);
    println!("{:?}", daemon.get_processes()["nginx"]);
}
