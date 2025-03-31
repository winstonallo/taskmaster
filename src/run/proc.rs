use std::{
    fs::File,
    os::unix::process::{CommandExt, ExitStatusExt},
    process::{Child, Command, ExitStatus},
    sync::Mutex,
    time::{self, Duration, Instant},
};

use crate::{
    conf::{self, proc::ProcessConfig},
    log_error, log_info, proc_info,
};
pub use error::ProcessError;
use libc::{c_int, signal, umask};

use super::statemachine::states::ProcessState;

mod error;

#[allow(unused)]
#[derive(Debug)]
pub struct Process {
    id: Option<u32>,
    name: String,
    child: Option<Child>,
    conf: ProcessConfig,
    startup_failures: u8,
    runtime_failures: u8,
    state: Mutex<ProcessState>,
}

impl Process {
    pub fn from_process_config(conf: conf::proc::ProcessConfig, proc_name: &str) -> Self {
        match conf.autostart() {
            true => Self {
                id: None,
                name: proc_name.to_string(),
                child: None,
                conf,
                startup_failures: 0,
                runtime_failures: 0,
                state: Mutex::new(ProcessState::Ready),
            },
            false => Self {
                id: None,
                name: proc_name.to_string(),
                child: None,
                conf,
                startup_failures: 0,
                runtime_failures: 0,
                state: Mutex::new(ProcessState::Idle),
            },
        }
    }
}

extern "C" fn kill(_signum: c_int) {
    unsafe {
        libc::_exit(1);
    }
}

#[allow(unused)]
impl Process {
    pub fn state(&self) -> ProcessState {
        self.state.lock().expect("something went terribly wrong").clone()
    }

    pub fn update_state(&mut self, new_state: ProcessState) {
        let mut handle = self.state.lock().expect("something went terribly wrong");
        *handle = new_state;
    }

    pub fn id(&self) -> Option<u32> {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn config(&self) -> &ProcessConfig {
        &self.conf
    }

    pub fn config_mut(&mut self) -> &mut ProcessConfig {
        &mut self.conf
    }

    pub fn runtime_failures(&self) -> u8 {
        self.runtime_failures
    }

    pub fn increment_runtime_failures(&mut self) {
        self.runtime_failures = self.runtime_failures.saturating_add(1);
    }

    pub fn startup_failures(&self) -> u8 {
        self.startup_failures
    }

    pub fn increment_startup_failures(&mut self) {
        self.startup_failures = self.startup_failures.saturating_add(1);
    }

    /// Returns a `time::Instant` representing the next time the process should
    /// be retried according to its configured backoff, assuming the failure
    /// happened at the time of calling this method.
    pub fn retry_at(&self) -> time::Instant {
        Instant::now() + Duration::from_secs(self.conf.backoff() as u64)
    }

    /// Checks whether the process is healthy, according to `started_at` and its
    /// configured healthcheck time.
    pub fn healthy(&self, started_at: time::Instant) -> bool {
        Instant::now().duration_since(started_at).as_secs() >= self.conf.starttime() as u64
    }

    fn spawn(&self) -> Result<Child, ProcessError> {
        let stdout_file = File::create(self.conf.stdout()).map_err(|err| ProcessError::Internal(err.to_string()))?;
        let stderr_file = File::create(self.conf.stderr()).map_err(|err| ProcessError::Internal(err.to_string()))?;

        let cmd_path = self.conf.cmd().path().to_owned();
        let args = self.conf.args().to_owned();
        let working_dir = self.conf.workingdir().path();
        let stop_signals = self.conf.stopsignals().to_owned();
        let umask_val  = self.conf.umask();

        match unsafe {
            Command::new(cmd_path)
                .args(args)
                .stdout(stdout_file)
                .stderr(stderr_file)
                .current_dir(working_dir)
                .pre_exec(move || {
                    for sig in &stop_signals {
                        signal(sig.signal(), kill as usize);
                    }
                    umask(umask_val.try_into().unwrap());
                    Ok(())
                })
                .spawn()
        } {
            Ok(child) => Ok(child),
            Err(err) => Err(ProcessError::CouldNotSpawn(err.to_string())),
        }
    }

    pub fn start(&mut self) -> Result<(), ProcessError> {
        assert_ne!(self.state(), ProcessState::Healthy);

        self.child = match self.spawn() {
            Ok(child) => Some(child),
            Err(err) => return Err(err),
        };

        self.id = Some(self.child.as_ref().unwrap().id());

        Ok(())
    }

    fn check_signal(&mut self, status: ExitStatus, pid: u32) -> Option<i32> {
        if let Some(signal) = status.signal() {
            log_info!("PID {} terminated by signal {}", pid, signal);
        } else {
            log_info!("PID {} terminated without exit or signal information", pid)
        }
        self.update_state(ProcessState::Stopped);
        None
    }

    pub fn exited(&mut self) -> Option<i32> {
        self.child.as_ref()?;

        let pid = self.id().expect("id should always be set if the program is running");

        match self.child.as_mut().unwrap().try_wait() {
            Ok(Some(status)) => match status.code() {
                Some(code) => {
                    self.child = None;
                    Some(code)
                }
                None => {
                    self.child = None;
                    self.check_signal(status, pid)
                }
            },
            Ok(None) => None,
            Err(err) => {
                log_error!("could not get status for PID {}: {}", self.id().expect("something went very wrong"), err);
                None
            }
        }
    }

    pub fn stop(&mut self) -> std::io::Result<()> {
        use ProcessState::*;
        match self.state() {
            HealthCheck(_) | Healthy => {
                self.child.take().unwrap().kill();
                proc_info!(
                    self.name(),
                    "killed, PID {}",
                    self.id().expect("process without id killed - this should not happen")
                );
                self.id.take();
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state() {
        let proc = Process {
            id: Some(1),
            name: ("".to_string()),
            child: None,
            conf: conf::proc::ProcessConfig::testconfig(),
            startup_failures: 0,
            runtime_failures: 0,
            state: Mutex::new(ProcessState::Idle),
        };
        assert_eq!(proc.state(), ProcessState::Idle)
    }

    #[test]
    fn update_state() {
        let mut proc = Process {
            id: Some(1),
            name: ("".to_string()),
            child: None,
            conf: conf::proc::ProcessConfig::testconfig(),
            startup_failures: 0,
            runtime_failures: 0,
            state: Mutex::new(ProcessState::Idle),
        };
        proc.update_state(ProcessState::Healthy);
        assert_eq!(proc.state(), ProcessState::Healthy)
    }
}
