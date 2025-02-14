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

impl Process {
    pub fn get_stdout(&mut self) -> &str {
        &self.stdout
    }

    pub fn get_stderr(&mut self) -> &str {
        &self.stderr
    }

    pub fn set_stdout(&mut self, path: &str) {
        self.stdout = path.to_string();
    }

    pub fn set_stderr(&mut self, path: &str) {
        self.stderr = path.to_string();
    }
}
