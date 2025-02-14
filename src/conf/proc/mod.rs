#[cfg(test)]
pub mod defaults;

#[cfg(not(test))]
mod defaults;

pub mod deserializers;

use serde::Deserialize;

#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
pub struct ProcessConfig {
    cmd: deserializers::path::ExecutableFile,

    #[serde(default = "defaults::dflt_processes")]
    processes: u8,

    #[serde(default = "defaults::dflt_umask")]
    umask: String,

    workingdir: deserializers::path::AccessibleDirectory,

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
    stdout: deserializers::path::WritableFile,

    #[serde(default = "defaults::dflt_stderr")]
    stderr: deserializers::path::WritableFile,

    env: Option<Vec<(String, String)>>,
}

#[allow(unused)]
impl ProcessConfig {
    pub fn cmd(&self) -> &deserializers::path::ExecutableFile {
        &self.cmd
    }

    pub fn processes(&self) -> u8 {
        self.processes
    }

    pub fn umask(&self) -> &str {
        &self.umask
    }

    pub fn workingdir(&self) -> &deserializers::path::AccessibleDirectory {
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
        &self.stdout.path()
    }

    pub fn stderr(&self) -> &str {
        &self.stderr.path()
    }

    pub fn env(&self) -> &Option<Vec<(String, String)>> {
        &self.env
    }

    pub fn set_stdout(&mut self, path: &str) {
        self.stdout = deserializers::path::WritableFile::default();
    }

    pub fn set_stderr(&mut self, path: &str) {
        self.stderr = deserializers::path::WritableFile::default();
    }

    #[cfg(test)]
    pub fn testconfig() -> Self {
        Self {
            cmd: deserializers::path::ExecutableFile::default(),
            processes: 1,
            umask: String::from("022"),
            workingdir: deserializers::path::AccessibleDirectory::default(),
            autostart: true,
            autorestart: deserializers::autorestart::AutoRestart::test_autorestart(),
            exitcodes: vec![0],
            startretries: 1,
            starttime: 5,
            stopsignals: vec![deserializers::stopsignal::StopSignal::SigTerm],
            stoptime: 5,
            stdout: deserializers::path::WritableFile::from_path("/tmp/taskmaster_test.stdout"),
            stderr: deserializers::path::WritableFile::from_path("/tmp/taskmaster_test.stderr"),
            env: None,
        }
    }
}
