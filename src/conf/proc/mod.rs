mod defaults;

use serde::Deserialize;

#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
pub struct Process {
    cmd: String,

    #[serde(default = "defaults::dflt_processes")]
    processes: u8,

    #[serde(default = "defaults::dflt_umask")]
    umask: String,

    workingdir: String,

    #[serde(default = "defaults::dflt_autostart")]
    autostart: bool,

    #[serde(default = "defaults::dflt_autorestart")]
    autorestart: String,

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
impl Process {
    pub fn get_cmd(&self) -> &str {
        &self.cmd
    }

    pub fn get_processes(&self) -> u8 {
        self.processes
    }

    pub fn get_umask(&self) -> &str {
        &self.umask
    }

    pub fn get_workingdir(&self) -> &str {
        &self.workingdir
    }

    pub fn get_autostart(&self) -> bool {
        self.autostart
    }

    pub fn get_autorestart(&self) -> &str {
        &self.autorestart
    }

    pub fn get_exitcodes(&self) -> &Vec<u8> {
        &self.exitcodes
    }

    pub fn get_startretries(&self) -> u8 {
        self.startretries
    }

    pub fn get_starttime(&self) -> u16 {
        self.starttime
    }

    pub fn get_stopsignals(&self) -> &Vec<String> {
        &self.stopsignals
    }

    pub fn get_stoptime(&self) -> u8 {
        self.stoptime
    }

    pub fn get_stdout(&self) -> &str {
        &self.stdout
    }

    pub fn get_stderr(&self) -> &str {
        &self.stderr
    }

    pub fn set_stdout(&mut self, path: &str) {
        self.stdout = path.to_string();
    }

    pub fn set_stderr(&mut self, path: &str) {
        self.stderr = path.to_string();
    }
}
