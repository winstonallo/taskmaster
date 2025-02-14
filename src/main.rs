mod config;

fn main() {
    let conf = config::Config::new("./config/example.toml");
    println!("{:?}", conf.get_processes()["nginx"]);
}
