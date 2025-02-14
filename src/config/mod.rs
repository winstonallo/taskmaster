use serde::Deserialize;
use std::{collections::HashMap, fs};

fn dflt_processes() -> u8 {
    1
}

fn dflt_umask() -> String {
    String::from("022")
}

fn dflt_autostart() -> bool {
    false
}

fn dflt_autorestart() -> String {
    String::from("no")
}

fn dflt_exitcodes() -> Vec<u8> {
    vec![0u8]
}

fn dflt_startretries() -> u8 {
    3
}

fn dflt_startttime() -> u16 {
    5
}

fn dflt_stopsignals() -> Vec<String> {
    vec![String::from("TERM")]
}

fn dflt_stoptime() -> u8 {
    5
}

fn dflt_stdout() -> String {
    String::from("")
}

fn dflt_stderr() -> String {
    String::from("")
}

#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
pub struct Process {
    cmd: String,

    #[serde(default = "dflt_processes")]
    processes: u8,

    #[serde(default = "dflt_umask")]
    umask: String,

    workingdir: String,

    #[serde(default = "dflt_autostart")]
    autostart: bool,

    #[serde(default = "dflt_autorestart")]
    autorestart: String,

    #[serde(default = "dflt_exitcodes")]
    exitcodes: Vec<u8>,

    #[serde(default = "dflt_startretries")]
    startretries: u8,

    #[serde(default = "dflt_startttime")]
    starttime: u16,

    #[serde(default = "dflt_stopsignals")]
    stopsignals: Vec<String>,

    #[serde(default = "dflt_stoptime")]
    stoptime: u8,

    #[serde(default = "dflt_stdout")]
    stdout: String,

    #[serde(default = "dflt_stderr")]
    stderr: String,

    env: Option<Vec<(String, String)>>,
}

pub struct Config {
    processes: HashMap<String, Process>,
}

impl Config {
    pub fn new(config_path: &str) -> Result<Self, String> {
        let conf_str = fs::read_to_string(config_path).expect("could not open config");

        let mut procs: HashMap<String, Process> = match toml::from_str(&conf_str) {
            Ok(procs) => procs,
            Err(err) => {
                return Err(format!("could not parse config at '{config_path}': {err}"));
            }
        };
        for (proc_name, proc) in &mut procs {
            if proc.stdout.is_empty() {
                proc.stdout = format!("/tmp/{proc_name}.stdout");
            }
            if proc.stderr.is_empty() {
                proc.stderr = format!("/tmp/{proc_name}.stdout");
            }
        }

        Ok(Config { processes: procs })
    }

    // TODO: better solution than clone
    pub fn get_processes(&self) -> HashMap<String, Process> {
        self.processes.clone()
    }
}
