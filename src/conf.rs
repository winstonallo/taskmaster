use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs,
};

use defaults::{dflt_authgroup, dflt_logfile, dflt_socketpath};

use proc::ProcessConfig;
use serde::Deserialize;

pub const PID_FILE_PATH: &str = "/tmp/taskmaster.pid";

pub mod defaults;
pub mod proc;
mod tests;

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Path to the socket used for communication between taskmaster and its client.
    ///
    /// Default:
    /// ```toml
    /// socketpath = "/tmp/taskmaster.sock"
    /// ```
    #[serde(skip)]
    socketpath: String,

    /// Name of the group to be used for authenticating the client (similarly to
    /// the docker group).
    ///
    /// Default:
    /// ```toml
    /// authgroup = "taskmaster"
    /// ```
    #[serde(skip)]
    authgroup: String,

    /// Path to the file the logs will be written to.
    ///
    /// Default:
    /// ```toml
    /// logfile = "/tmp/tasmaster.log"
    /// ```
    #[serde(skip)]
    logfile: String,

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
                return Err(format!("could not parse config at path '{path}': '{err}'").into());
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

        conf.authgroup = dflt_authgroup();
        conf.socketpath = dflt_socketpath();
        conf.logfile = dflt_logfile();

        if conf.processes.is_empty() {
            return Err("taskmaster expects at least one process to be defined to operate".into());
        }

        let mut seen = HashSet::new();
        let duplicates = conf
            .processes
            .iter()
            .filter(|p| p.1.stdout().is_some())
            .map(|p| p.1.stdout().as_ref().unwrap().path().to_owned())
            .filter(|path| !seen.insert(path.clone()))
            .collect::<HashSet<String>>();

        if !duplicates.is_empty() {
            return Err(Box::<dyn Error>::from(format!("Found duplicated stdout paths: {duplicates:?}")));
        }

        let mut seen = HashSet::new();
        let duplicates = conf
            .processes
            .iter()
            .filter(|p| p.1.stderr().is_some())
            .map(|p| p.1.stderr().as_ref().unwrap().path().to_owned())
            .filter(|path| !seen.insert(path.clone()))
            .collect::<HashSet<String>>();

        if !duplicates.is_empty() {
            return Err(Box::<dyn Error>::from(format!("Found duplicated stderr paths: {duplicates:?}")));
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

    pub fn logfile(&self) -> &str {
        &self.logfile
    }
}

#[cfg(test)]
impl Default for Config {
    fn default() -> Self {
        Self {
            socketpath: dflt_socketpath(),
            authgroup: dflt_authgroup(),
            logfile: dflt_logfile(),
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
        self.authgroup = authgroup.into();
        self
    }

    pub fn add_process(&mut self, name: &str, process: ProcessConfig) -> &mut Self {
        self.processes.insert(name.to_string(), process);
        self
    }

    #[cfg(test)]
    pub fn random() -> Config {
        use rand::{Rng, distr::Alphanumeric};

        let socketpath = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect::<String>();

        Self::default().set_socketpath(&format!("/tmp/{socketpath}.sock")).to_owned()
    }
}
