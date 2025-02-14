#[cfg(test)]
pub mod defaults;

#[cfg(not(test))]
mod defaults;

use serde::{Deserialize, Deserializer};

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct AutoRestart {
    mode: String,
    max_retries: Option<u8>,
}

#[allow(unused)]
impl AutoRestart {
    pub fn mode(&self) -> &str {
        &self.mode
    }

    pub fn max_retries(&self) -> Option<u8> {
        self.max_retries
    }
}

impl<'de> Deserialize<'de> for AutoRestart {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "no" | "always" => Ok(Self { mode: s, max_retries: None }),
            _ if s.starts_with("on-failure[:") && s.ends_with("]") => {
                let max_retries_str = &s[12..s.len() - 1];
                let max_retries = match max_retries_str.parse::<u8>() {
                    Ok(n) => n,
                    Err(e) => {
                        return Err(serde::de::Error::custom(format!(
                            "invalid max-retries value for on-failure: {max_retries_str}: {e} (expected u8)"
                        )));
                    }
                };
                Ok(Self {
                    mode: String::from("on-failure"),
                    max_retries: Some(max_retries),
                })
            }
            _ => Err(serde::de::Error::custom(format!("invalid value for field 'autorestart': '{s}'"))),
        }
    }
}

#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
pub struct ProcessConfig {
    cmd: String,

    #[serde(default = "defaults::dflt_processes")]
    processes: u8,

    #[serde(default = "defaults::dflt_umask")]
    umask: String,

    workingdir: String,

    #[serde(default = "defaults::dflt_autostart")]
    autostart: bool,

    #[serde(default = "defaults::dflt_autorestart")]
    autorestart: AutoRestart,

    #[serde(default = "defaults::dflt_exitcodes")]
    exitcodes: Vec<u8>,

    #[serde(default = "defaults::dflt_startretries")]
    startretries: u8,

    #[serde(default = "defaults::dflt_startttime")]
    starttime: u16,

    #[serde(default = "defaults::dflt_stopsignals")]
    stopsignals: Vec<String>,

    #[serde(default = "defaults::dflt_stoptime")]
    stoptime: u8,

    #[serde(default = "defaults::dflt_stdout")]
    stdout: String,

    #[serde(default = "defaults::dflt_stderr")]
    stderr: String,

    env: Option<Vec<(String, String)>>,
}

#[allow(unused)]
impl ProcessConfig {
    pub fn cmd(&self) -> &str {
        &self.cmd
    }

    pub fn processes(&self) -> u8 {
        self.processes
    }

    pub fn umask(&self) -> &str {
        &self.umask
    }

    pub fn workingdir(&self) -> &str {
        &self.workingdir
    }

    pub fn autostart(&self) -> bool {
        self.autostart
    }

    pub fn autorestart(&self) -> &AutoRestart {
        &self.autorestart
    }

    pub fn exitcodes(&self) -> &Vec<u8> {
        &self.exitcodes
    }

    pub fn startretries(&self) -> u8 {
        self.startretries
    }

    pub fn starttime(&self) -> u16 {
        self.starttime
    }

    pub fn stopsignals(&self) -> &Vec<String> {
        &self.stopsignals
    }

    pub fn stoptime(&self) -> u8 {
        self.stoptime
    }

    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    pub fn stderr(&self) -> &str {
        &self.stderr
    }

    pub fn env(&self) -> &Option<Vec<(String, String)>> {
        &self.env
    }

    pub fn set_stdout(&mut self, path: &str) {
        self.stdout = path.to_string();
    }

    pub fn set_stderr(&mut self, path: &str) {
        self.stderr = path.to_string();
    }

    #[cfg(test)]
    pub fn testconfig() -> Self {
        Self {
            cmd: String::from("echo"),
            processes: 1,
            umask: String::from("022"),
            workingdir: String::from("/tmp"),
            autostart: true,
            autorestart: AutoRestart {
                mode: String::from("no"),
                max_retries: None,
            },
            exitcodes: vec![0],
            startretries: 1,
            starttime: 5,
            stopsignals: vec![String::from("TERM")],
            stoptime: 5,
            stdout: String::from("/tmp/taskmaster_test.stdout"),
            stderr: String::from("/tmp/taskmaster_test.stderr"),
            env: None,
        }
    }
}
