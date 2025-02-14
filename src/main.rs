mod conf;

fn main() {
    let conf = conf::Config::from_file("./config/example.toml").expect("config construction failed");
    println!("{:?}", conf.get_processes()["nginx"]);
}
