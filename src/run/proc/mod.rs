use std::{
    fs::File,
    process::{Child, Command},
    sync::{Arc, Mutex},
    thread,
    time::{self, Duration, Instant},
};

use crate::conf::{self, proc::ProcessConfig};
pub use error::ProcessError;
use libc::umask;

mod error;

#[allow(unused)]
#[derive(Debug)]
pub struct Process<'tm> {
    id: Option<u32>,
    name: String,
    child: Option<Child>,
    conf: &'tm ProcessConfig,
    last_startup_try: Option<time::Instant>,
    startup_tries: u8,
    status: Mutex<ProcessStatus>,
}

impl<'tm> Process<'tm> {
    pub fn from_process_config(conf: &'tm conf::proc::ProcessConfig, name: &str) -> Self {
        Self {
            id: None,
            name: name.to_string(),
            child: None,
            conf,
            last_startup_try: None,
            startup_tries: 0,
            status: Mutex::new(ProcessStatus::NotStarted),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ProcessStatus {
    Exited(i32),
    Running,
    NotStarted,
    HealthCheck,
    Restarting(u8),
}

#[allow(unused)]
impl<'tm> Process<'tm> {
    /// Thread-safe getter for `self.status`.
    fn status(&self) -> ProcessStatus {
        *self.status.lock().expect("something went terribly wrong")
    }

    /// Thread-safe setter for `self.status`.
    fn set_status(&mut self, status: ProcessStatus) {
        let current_status = self.status.get_mut().expect("something went terribly wrong");
        *current_status = status
    }

    pub fn id(&self) -> Option<u32> {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn running(&self) -> bool {
        self.id.is_some() && self.child.is_some()
    }

    pub fn healthy(&self) -> bool {
        self.status() == ProcessStatus::Running
    }

    pub fn config(&self) -> &'tm ProcessConfig {
        self.conf
    }

    pub fn last_startup_try(&self) -> Option<time::Instant> {
        self.last_startup_try
    }

    pub fn failed(&self) -> bool {
        self.startup_tries == self.config().startretries()
    }

    fn restart_in(&self, exit_code: i32) -> Option<u8> {
        match self.config().autorestart().mode() {
            "always" => Some(self.config().backoff()),
            "on-failure" => {
                if !self.config().exitcodes().contains(&exit_code) {
                    Some(self.config().backoff())
                } else {
                    None
                }
            }
            "no" => None,
            _ => panic!("error in config parsing"),
        }
    }

    pub fn exited(&mut self) -> Result<ProcessStatus, ProcessError> {
        if !self.running() {
            return Ok(ProcessStatus::NotStarted);
        }

        match self.child.as_mut().expect("child should definitely be Some here").try_wait() {
            Ok(ex) => match ex {
                Some(status) => {
                    self.child = None;
                    self.id = None;
                    let code = status.code().unwrap_or(0);
                    if !self.config().exitcodes().contains(&code) {
                        self.startup_tries += 1;

                        match self.restart_in(code) {
                            Some(secs) => {}
                            None => {}
                        }

                        let restart_time = match self.config().autorestart().mode() {
                            "always" => 0,
                            "on-failure" => self
                                .config()
                                .autorestart()
                                .max_retries()
                                .expect("max_retries should always be set if mode == \"on-failure\""),
                            "no" => {
                                return Ok(ProcessStatus::Exited(status.code().unwrap_or(0)));
                            }
                            _ => panic!("error in config parsing"),
                        };
                        let last_try_t = self.last_startup_try.expect("last_startup_try should be set if the process is running");
                        let wait_time = ((last_try_t + Duration::from_secs(restart_time as u64) - last_try_t).as_secs()).max(0);
                        Ok(ProcessStatus::Restarting(
                            wait_time.try_into().expect("wait_time not fitting into a u8 means error in config parsing"),
                        ))
                    } else {
                        Ok(ProcessStatus::Exited(status.code().unwrap_or(0)))
                    }
                    // println!("process {} exited with status: {}", self.name, status.code().unwrap_or(0));
                }
                None => Ok(ProcessStatus::Running),
            },
            Err(err) => Err(ProcessError::Status(err.to_string())),
        }
    }

    pub fn start(&mut self) -> Result<(), ProcessError> {
        if self.startup_tries == self.config().startretries() {
            return Err(ProcessError::MaxRetriesReached(format!("tried {} times", self.startup_tries)));
        }

        // If the healthcheck is currently running, do not try to start
        if self.running() || self.status() == ProcessStatus::HealthCheck {
            return Ok(());
        }

        let current_umask = unsafe { umask(0) };

        unsafe { umask(self.conf.umask()) };

        let stdout_file = File::create(self.conf.stdout()).map_err(|err| ProcessError::Internal(err.to_string()))?;
        let stderr_file = File::create(self.conf.stderr()).map_err(|err| ProcessError::Internal(err.to_string()))?;

        self.child = match Command::new(self.conf.cmd().path())
            .stdout(stdout_file)
            .stderr(stderr_file)
            .args(self.conf.args())
            .current_dir(self.conf.workingdir().path())
            .spawn()
        {
            Ok(child) => Some(child),
            Err(err) => {
                self.startup_tries += 1;
                self.last_startup_try = Some(time::Instant::now());
                // let last_try_t = self.last_startup_try.expect("last_startup_try should be set if the process is running");
                // let wait_time = ((last_try_t + Duration::from_secs(restart_time as u64) - last_try_t).as_secs()).max(0);
                // self.status = ProcessStatus::Restarting(wait_time.try_into().expect("wait_time not fitting into a u8 means error in config parsing"));
                return Err(ProcessError::StartUp(err.to_string()));
            }
        };

        self.set_status(ProcessStatus::HealthCheck);

        unsafe { umask(current_umask) };

        self.last_startup_try = Some(time::Instant::now());
        self.id = Some(self.child.as_ref().unwrap().id());

        let process_arc = Arc::new(Mutex::new(self));
        let process_clone = Arc::clone(&process_arc);
        thread::spawn(move || {
            let mut process = process_clone.lock().unwrap();
            process.healthcheck()
        });

        Ok(())
    }

    fn healthcheck(&mut self) -> ! {
        loop {
            match self
                .child
                .as_mut()
                .expect("healthcheck should never be called on non-running process")
                .try_wait()
            {
                Ok(status) => match status {
                    Some(code) => match self.restart_in(code.code().unwrap_or(0)) {
                        Some(time) => self.set_status(ProcessStatus::Restarting(time)),
                        None => self.set_status(ProcessStatus::Exited(code.code().unwrap_or(0))),
                    },
                    None => {}
                },
                Err(err) => {}
            }
        }
    }

    pub fn stop(&mut self) -> std::io::Result<()> {
        if !self.running() {
            return Ok(());
        }

        self.child.take().unwrap().kill();
        self.id.take();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::process::Stdio;

    use super::*;

    #[test]
    fn running_no_id() {
        let proc = Process {
            id: None,
            name: String::from(""),
            child: None,
            conf: &conf::proc::ProcessConfig::testconfig(),
            last_startup_try: None,
            startup_tries: 0,
            status: ProcessStatus::NotStarted.into(),
        };

        assert!(!proc.running())
    }

    #[test]
    fn running_has_id() {
        let proc = Process {
            id: Some(1),
            name: String::from(""),
            child: Some(Command::new("/bin/ls").stdout(Stdio::null()).spawn().expect("could not run command")),
            conf: &conf::proc::ProcessConfig::testconfig(),
            last_startup_try: None,
            startup_tries: 0,
            status: ProcessStatus::NotStarted.into(),
        };

        assert!(proc.running())
    }
}
