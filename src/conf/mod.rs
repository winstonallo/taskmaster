use std::{collections::HashMap, fs};

use proc::Process;

mod proc;

pub struct Config {
    processes: HashMap<String, Process>,
}

impl Config {
    pub fn from_file(config_path: &str) -> Result<Self, String> {
        let conf_str = fs::read_to_string(config_path).expect("could not open config");

        let mut procs: HashMap<String, Process> = match toml::from_str(&conf_str) {
            Ok(procs) => procs,
            Err(err) => {
                return Err(format!("could not parse config at '{config_path}': {err}"));
            }
        };

        for (proc_name, proc) in &mut procs {
            if proc.get_stdout().is_empty() {
                proc.set_stdout(&format!("/tmp/{proc_name}.stdout"));
            }
            if proc.get_stderr().is_empty() {
                proc.set_stderr(&format!("/tmp/{proc_name}.stdout"));
            }
        }

        Ok(Config { processes: procs })
    }

    pub fn get_processes(&self) -> &HashMap<String, Process> {
        &self.processes
    }
}
