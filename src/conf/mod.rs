use std::{collections::HashMap, fs};

use proc::ProcessConfig;

pub mod proc;

pub struct Config {
    processes: HashMap<String, ProcessConfig>,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self, String> {
        let conf_str = fs::read_to_string(path).expect("could not open config");

        let mut procs: HashMap<String, ProcessConfig> = match toml::from_str(&conf_str) {
            Ok(procs) => procs,
            Err(err) => {
                return Err(format!("could not parse config at '{path}': {err}"));
            }
        };

        for (proc_name, proc) in &mut procs {
            if proc.stdout().is_empty() {
                proc.set_stdout(&format!("/tmp/{proc_name}.stdout"));
            }
            if proc.stderr().is_empty() {
                proc.set_stderr(&format!("/tmp/{proc_name}.stdout"));
            }
        }

        Ok(Config { processes: procs })
    }

    pub fn get_processes(&self) -> &HashMap<String, ProcessConfig> {
        &self.processes
    }
}
