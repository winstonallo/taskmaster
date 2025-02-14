mod config;

fn main() {
    let conf = config::Config::new("./config/example.toml").expect("config construction failed");
    println!("{:?}", conf.get_processes()["nginx"]);
}
