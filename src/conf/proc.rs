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
use types::HealthCheck;

/// # ProcessConfig
/// `src/conf/proc/mod.rs`
///
/// Rust representation of the `taskmaster` config. All its types implement
/// the `Deserializer` trait.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessConfig {
    /// Command to run in order to start this process.
    ///
    /// ```toml
    /// [processes.nginx]
    /// cmd = "/usr/sbin/nginx"
    /// workingdir = "/var/www"
    /// ```
    ///
    /// Required.
    cmd: types::ExecutableFile,

    /// Args to pass to `cmd`.
    ///
    /// ```toml
    /// [processes.nginx]
    /// cmd = "/usr/sbin/nginx"
    /// workingdir = "/var/www"
    /// args = ["-v"]
    /// ```
    ///
    /// Defaults to no arguments.
    #[serde(default = "defaults::dflt_args")]
    args: Vec<String>,

    /// Number of copies of this process to start.
    ///
    /// ```toml
    /// [processes.nginx]
    /// cmd = "/usr/sbin/nginx"
    /// workingdir = "/var/www"
    /// processes = 4
    /// ```
    ///
    /// Defaults to `1`.
    #[serde(default = "defaults::dflt_processes")]
    processes: u8,

    /// Mask for files created by the process.
    ///
    /// ```toml
    /// [processes.nginx]
    /// cmd = "/usr/sbin/nginx"
    /// workingdir = "/var/www"
    /// umask = 011
    /// ```
    ///
    /// Defaults to `022`.
    #[serde(default = "defaults::dflt_umask")]
    umask: types::Umask,

    /// Working directory for the process. Must be an absolute path.
    ///
    /// ```toml
    /// [processes.nginx]
    /// cmd = "/usr/sbin/nginx"
    /// workingdir = "/tmp"
    /// ```
    ///
    /// Required.
    workingdir: types::AccessibleDirectory,

    /// Whether to start the process automatically when starting `taskmaster`.
    ///
    /// ```toml
    /// [processes.nginx]
    /// cmd = "/usr/sbin/nginx"
    /// workingdir = "/var/www"
    /// autostart = true
    /// ```
    ///
    /// Defaults to `false`.
    #[serde(default = "defaults::dflt_autostart")]
    autostart: bool,

    /// Restart the process when it quits, options are:
    /// - `no`: Never restart the process automatically.
    /// - `on-failure[:max-retries]`: Try to restart the process `max-retries` times
    ///   when it exits unexpectedly.
    /// - `always`: Always restart when exiting.
    ///
    /// ```toml
    /// [processes.nginx]
    /// cmd = "/usr/sbin/nginx"
    /// workingdir = "/var/www"
    /// autorestart = on-failure[:5]
    /// ```
    ///
    /// Defaults to `no`.
    #[serde(default = "defaults::dflt_autorestart")]
    autorestart: types::AutoRestart,

    /// List of exit codes to be interpreted as successful.
    ///
    /// ```toml
    /// [processes.nginx]
    /// cmd = "/usr/sbin/nginx"
    /// workingdir = "/var/www"
    /// exitcodes = [0, 2, 42]
    /// ```
    ///
    /// Defaults to `[0]`.
    #[serde(default = "defaults::dflt_exitcodes")]
    exitcodes: Vec<i32>,

    /// Healthcheck configuration. See [`types::HealthCheck`] for
    /// more details.
    ///
    /// Must be one of:
    /// - Command HealthCheck:
    /// ```toml
    /// cmd = <string>
    /// args = <<string>>
    /// ```
    /// - Uptime HealthCheck:
    ///
    /// ```toml
    /// starttime = <int>
    /// ```
    ///
    /// Defaults to `{starttime = 5, retries = 5, backoff = 5}`
    #[serde(default)]
    healthcheck: HealthCheck,

    /// List of signals triggering able to stop the process, options are any
    /// FreeBSD signal (<https://www.math.stonybrook.edu/~ccc/dfc/dfc/signals.html>).
    ///
    /// ```toml
    /// [processes.nginx]
    /// cmd = "/usr/sbin/nginx"
    /// workingdir = "/var/www"
    /// stopsignals = ["TERM", "USR1"]
    /// ```
    ///
    /// Defaults to `["TERM"]`.
    #[serde(default = "defaults::dflt_stopsignals")]
    stopsignals: Vec<types::StopSignal>,

    /// Time (in seconds) to wait for the process to stop. If it does not stop within
    /// this time, it will be killed forcibly.
    ///
    /// ```toml
    /// [processes.nginx]
    /// cmd = "/usr/sbin/nginx"
    /// workingdir = "/var/www"
    /// stoptime = 20
    /// ```
    ///
    /// Defaults to `5`, max `255`.
    #[serde(default = "defaults::dflt_stoptime")]
    stoptime: u8,

    /// File the standard output of the process should be redirected to.
    ///
    /// ```toml
    /// [processes.nginx]
    /// cmd = "/usr/sbin/nginx"
    /// workingdir = "/var/www"
    /// stdout = "nginx.stdout"
    /// ```
    ///
    /// Defaults to `/tmp/<process_name>.stdout`.
    #[serde(default = "defaults::dflt_stdout")]
    stdout: types::WritableFile,

    /// File the standard error of the process should be redirected to.
    ///
    /// ```toml
    /// [processes.nginx]
    /// cmd = "/usr/sbin/nginx"
    /// workingdir = "/var/www"
    /// stderr = "nginx.stderr"
    /// ```
    ///
    /// Defaults to `/tmp/<process_name>.stderr`.
    #[serde(default = "defaults::dflt_stderr")]
    stderr: types::WritableFile,

    /// Key value pairs of environment variables to be injected into the process
    /// at startup.
    ///
    /// ```toml
    /// [processes.nginx]
    /// cmd = "/usr/sbin/nginx"
    /// workingdir = "/var/www"
    /// env = [["HOME", "/home/abied-ch"], ["ANSWER", "42"]]
    /// ```
    ///
    /// Defaults to an empty list.
    #[serde(default)]
    env: Vec<(String, String)>,
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

    pub fn exitcodes(&self) -> &Vec<i32> {
        &self.exitcodes
    }

    pub fn healthcheck(&self) -> &HealthCheck {
        &self.healthcheck
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

    pub fn env(&self) -> &Vec<(String, String)> {
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
            exitcodes: vec![0],
            healthcheck: HealthCheck::default(),
            stopsignals: vec![types::StopSignal(SIGTERM)],
            stoptime: 5,
            stdout: types::WritableFile::from_path("/tmp/taskmaster_test.stdout"),
            stderr: types::WritableFile::from_path("/tmp/taskmaster_test.stderr"),
            env: Vec::new(),
        }
    }
}
