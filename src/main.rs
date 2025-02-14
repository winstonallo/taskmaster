mod conf;

fn main() {
    let conf = conf::Config::new("./config/example.toml").expect("config construction failed");
    println!("{:?}", conf.get_processes()["nginx"]);
}
