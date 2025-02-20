use std::{collections::HashMap, fs};

use proc::ProcessConfig;
use serde::Deserialize;

mod defaults;
pub mod proc;
mod tests;

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "defaults::dflt_socketpath")]
    socketpath: String,
    #[serde(default = "defaults::dflt_authgroup")]
    authgroup: String,
    #[serde(flatten)]
    processes: HashMap<String, ProcessConfig>,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self, String> {
        let conf_str = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(err) => {
                return Err(format!("could not read config at path '{path}' to into string: {err}"));
            }
        };

        Config::parse(&conf_str)
    }

    #[cfg(test)]
    pub fn from_str(config: &str) -> Result<Self, String> {
        Config::parse(config)
    }

    fn parse(config_str: &str) -> Result<Self, String> {
        let mut conf: Config = match toml::from_str(config_str) {
            Ok(cnf) => cnf,
            Err(err) => {
                return Err(format!("could not parse config string: {err}"));
            }
        };

        // Did not find a way to have serde defaults depend on other field's values.
        for (proc_name, proc) in &mut conf.processes {
            if proc.stdout().is_empty() {
                proc.set_stdout(&format!("/tmp/{proc_name}.stdout"));
            }
            if proc.stderr().is_empty() {
                proc.set_stderr(&format!("/tmp/{proc_name}.stderr"));
            }
        }

        Ok(conf)
    }

    pub fn processes(&self) -> &HashMap<String, ProcessConfig> {
        &self.processes
    }

    pub fn socketpath(&self) -> &str {
        &self.socketpath
    }

    pub fn authgroup(&self) -> &str {
        &self.authgroup
    }
}
