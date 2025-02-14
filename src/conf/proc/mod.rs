#[cfg(test)]
pub mod defaults;

#[cfg(not(test))]
mod defaults;

pub mod deserializers;

use serde::Deserialize;

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
    autorestart: deserializers::autorestart::AutoRestart,

    #[serde(default = "defaults::dflt_exitcodes")]
    exitcodes: Vec<u8>,

    #[serde(default = "defaults::dflt_startretries")]
    startretries: u8,

    #[serde(default = "defaults::dflt_startttime")]
    starttime: u16,

    #[serde(default = "defaults::dflt_stopsignals")]
    stopsignals: Vec<deserializers::stopsignal::StopSignal>,

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

    pub fn autorestart(&self) -> &deserializers::autorestart::AutoRestart {
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

    pub fn stopsignals(&self) -> &Vec<deserializers::stopsignal::StopSignal> {
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
            autorestart: deserializers::autorestart::AutoRestart::test_autorestart(),
            exitcodes: vec![0],
            startretries: 1,
            starttime: 5,
            stopsignals: vec![deserializers::stopsignal::StopSignal::SigTerm],
            stoptime: 5,
            stdout: String::from("/tmp/taskmaster_test.stdout"),
            stderr: String::from("/tmp/taskmaster_test.stderr"),
            env: None,
        }
    }
}
