use std::{collections::HashMap, error::Error, fs};

use proc::ProcessConfig;
use serde::Deserialize;

mod defaults;
pub mod proc;
mod tests;

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Path to the socket used for communication between taskmaster and its client.
    ///
    /// Default:
    /// ```toml
    /// socketpath = "/tmp/.taskmaster.sock"
    /// ```
    #[serde(default = "defaults::dflt_socketpath")]
    socketpath: String,

    /// Name of the group to be used for authenticating the client (similarly to
    /// the docker group).
    ///
    /// Default:
    /// ```toml
    /// authgroup = "taskmaster"
    /// ```
    #[serde(default = "defaults::dflt_authgroup")]
    authgroup: String,

    /// Map of processes to configure individually. For process-level configuration,
    /// see [`crate::conf::proc::ProcessConfig`].
    ///
    /// Example:
    /// ```toml
    /// [processes.nginx]
    /// cmd = "/usr/sbin/nginx"
    /// workingdir = "/var/www/"
    /// ```
    /// At least one process must be defined for `taskmaster`to run.
    #[serde(default)]
    processes: HashMap<String, ProcessConfig>,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self, Box<dyn Error>> {
        let conf_str = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(err) => {
                return Err(format!("could not parse config at path '{}': '{}'", path, err).into());
            }
        };

        Config::parse(&conf_str)
    }

    #[cfg(test)]
    pub fn from_str(config: &str) -> Result<Self, Box<dyn Error>> {
        Config::parse(config)
    }

    fn parse(config_str: &str) -> Result<Self, Box<dyn Error>> {
        let mut conf: Config = match toml::from_str(config_str) {
            Ok(cnf) => cnf,
            Err(err) => {
                return Err(err.into());
            }
        };

        if conf.processes.is_empty() {
            return Err("taskmaster expects at least one process to be defined to operate".into());
        }

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

#[cfg(test)]
impl Default for Config {
    fn default() -> Self {
        Self {
            socketpath: "socketpath".to_string(),
            authgroup: "authgroup".to_string(),
            processes: HashMap::new(),
        }
    }
}

#[cfg(test)]
impl Config {
    pub fn set_socketpath(&mut self, socketpath: &str) -> &mut Self {
        self.socketpath = socketpath.to_string();
        self
    }

    pub fn set_authgroup(&mut self, authgroup: &str) -> &mut Self {
        self.authgroup = authgroup.to_string();
        self
    }

    pub fn add_process(&mut self, name: &str, process: ProcessConfig) -> &mut Self {
        self.processes.insert(name.to_string(), process);
        self
    }

    #[cfg(test)]
    pub fn random() -> Config {
        use rand::{Rng, distr::Alphanumeric};

        let socketpath = rand::rng().sample_iter(&Alphanumeric).take(8).map(char::from).collect::<String>();

        Self::default().set_socketpath(&format!("/tmp/{socketpath}.sock")).to_owned()
    }
}
