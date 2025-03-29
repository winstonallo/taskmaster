#[cfg(test)]
/// # defaults
/// `src/conf/proc/defaults.ts`
///
/// Contains functions returning the configuration fields' default values, passed to
/// `serde` in case a non-required field is empty.  
pub mod defaults;

#[cfg(not(test))]
/// # defaults
/// `src/conf/proc/defaults.ts`
///
/// Contains functions returning the configuration fields' default values, passed to
/// `serde` in case a non-required field is empty.  
mod defaults;

/// # types
/// `src/conf/proc/types`
///
/// Contains types used for loading non-primitively typed configuration fields,
/// which may required custom validation rules such as path validation or value checking
/// going beyond simple overflow checks (which are handled by `serde`).
///
/// Each of those types enforces its custom rules by implementing the `Deserializer` trait,
/// allowing them to be directly deserialized into `ProcessConfig`.
pub mod types;

use serde::Deserialize;

/// # ProcessConfig
/// `src/conf/proc/mod.rs`
///
/// Rust representation of the `taskmaster` config. All its types implement
/// the `Deserializer` trait.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
pub struct ProcessConfig {
    cmd: types::ExecutableFile,

    #[serde(default = "defaults::dflt_args")]
    args: Vec<String>,

    #[serde(default = "defaults::dflt_processes")]
    processes: u8,

    #[serde(default = "defaults::dflt_umask")]
    umask: types::Umask,

    workingdir: types::AccessibleDirectory,

    #[serde(default = "defaults::dflt_autostart")]
    autostart: bool,

    #[serde(default = "defaults::dflt_autorestart")]
    autorestart: types::AutoRestart,

    #[serde(default = "defaults::dflt_backoff")]
    backoff: u8,

    #[serde(default = "defaults::dflt_exitcodes")]
    exitcodes: Vec<i32>,

    #[serde(default = "defaults::dflt_startretries")]
    startretries: u8,

    #[serde(default = "defaults::dflt_startttime")]
    starttime: u16,

    #[serde(default = "defaults::dflt_stopsignals")]
    stopsignals: Vec<types::StopSignal>,

    #[serde(default = "defaults::dflt_stoptime")]
    stoptime: u8,

    #[serde(default = "defaults::dflt_stdout")]
    stdout: types::WritableFile,

    #[serde(default = "defaults::dflt_stderr")]
    stderr: types::WritableFile,

    env: Option<Vec<(String, String)>>,
}

#[allow(unused)]
impl ProcessConfig {
    pub fn cmd(&self) -> &types::ExecutableFile {
        &self.cmd
    }

    pub fn args(&self) -> &Vec<String> {
        &self.args
    }

    pub fn processes(&self) -> u8 {
        self.processes
    }

    pub fn umask(&self) -> u32 {
        self.umask.mask()
    }

    pub fn workingdir(&self) -> &types::AccessibleDirectory {
        &self.workingdir
    }

    pub fn autostart(&self) -> bool {
        self.autostart
    }

    pub fn autorestart(&self) -> &types::AutoRestart {
        &self.autorestart
    }

    pub fn backoff(&self) -> u8 {
        assert_eq!(self.autorestart.mode(), "on-failure");

        self.backoff
    }

    pub fn exitcodes(&self) -> &Vec<i32> {
        &self.exitcodes
    }

    pub fn startretries(&self) -> u8 {
        self.startretries
    }

    pub fn starttime(&self) -> u16 {
        self.starttime
    }

    pub fn stopsignals(&self) -> &Vec<types::StopSignal> {
        &self.stopsignals
    }

    pub fn stoptime(&self) -> u8 {
        self.stoptime
    }

    pub fn stdout(&self) -> &str {
        self.stdout.path()
    }

    pub fn stderr(&self) -> &str {
        self.stderr.path()
    }

    pub fn env(&self) -> &Option<Vec<(String, String)>> {
        &self.env
    }

    pub fn set_stdout(&mut self, path: &str) {
        self.stdout = types::WritableFile::from_path(path);
    }

    pub fn set_stderr(&mut self, path: &str) {
        self.stderr = types::WritableFile::from_path(path);
    }

    #[cfg(test)]
    pub fn testconfig() -> Self {
        use libc::SIGTERM;

        Self {
            cmd: types::ExecutableFile::default(),
            args: defaults::dflt_args(),
            processes: 1,
            umask: types::Umask::default(),
            workingdir: types::AccessibleDirectory::default(),
            autostart: true,
            autorestart: types::AutoRestart::default(),
            backoff: 5,
            exitcodes: vec![0],
            startretries: 1,
            starttime: 5,
            stopsignals: vec![types::StopSignal(SIGTERM)],
            stoptime: 5,
            stdout: types::WritableFile::from_path("/tmp/taskmaster_test.stdout"),
            stderr: types::WritableFile::from_path("/tmp/taskmaster_test.stderr"),
            env: None,
        }
    }
}
