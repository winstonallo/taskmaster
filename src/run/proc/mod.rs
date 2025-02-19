use std::{
    fs::File,
    os::unix::process::{CommandExt, ExitStatusExt},
    process::{Child, Command},
    sync::Mutex,
    time,
};

use crate::{
    conf::{self, proc::ProcessConfig},
    log_error, log_info,
};
pub use error::ProcessError;
use libc::{c_int, signal, umask};
pub use state::ProcessState;

mod error;
pub mod state;

#[allow(unused)]
#[derive(Debug)]
pub struct Process<'tm> {
    id: Option<u32>,
    name: String,
    child: Option<Child>,
    conf: &'tm ProcessConfig,
    last_startup: Option<time::Instant>,
    startup_failures: u8,
    runtime_failures: u8,
    state: Mutex<ProcessState>,
}

impl<'tm> Process<'tm> {
    pub fn from_process_config(conf: &'tm conf::proc::ProcessConfig, proc_name: &str) -> Self {
        Self {
            id: None,
            name: proc_name.to_string(),
            child: None,
            conf,
            last_startup: None,
            startup_failures: 0,
            runtime_failures: 0,
            state: Mutex::new(ProcessState::Idle),
        }
    }
}

extern "C" fn kill(_signum: c_int) {
    unsafe {
        libc::_exit(1);
    }
}

#[allow(unused)]
impl Process<'_> {
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

    pub fn running(&self) -> bool {
        self.id.is_some() && self.child.is_some()
    }

    pub fn config(&self) -> &ProcessConfig {
        self.conf
    }

    pub fn last_startup(&self) -> Option<time::Instant> {
        self.last_startup
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

    pub fn start(&mut self) -> Result<(), ProcessError> {
        assert_ne!(self.state(), ProcessState::Running);

        let stdout_file = File::create(self.conf.stdout()).map_err(|err| ProcessError::Internal(err.to_string()))?;
        let stderr_file = File::create(self.conf.stderr()).map_err(|err| ProcessError::Internal(err.to_string()))?;

        let cmd_path = self.conf.cmd().path().to_owned();
        let args = self.conf.args().to_owned();
        let working_dir = self.conf.workingdir().path().to_owned();
        let stop_signals = self.config().stopsignals().to_owned();
        let umask_val = self.conf.umask();

        self.child = match unsafe {
            Command::new(cmd_path)
                .stdout(stdout_file)
                .stderr(stderr_file)
                .args(args)
                .current_dir(working_dir)
                .pre_exec(move || {
                    for sig in &stop_signals {
                        signal(sig.signal(), kill as usize);
                    }
                    unsafe { umask(umask_val) };
                    Ok(())
                })
                .spawn()
        } {
            Ok(child) => Some(child),
            Err(err) => {
                self.startup_failures += 1;
                return Err(ProcessError::CouldNotStartUp(err.to_string()));
            }
        };

        self.id = Some(self.child.as_ref().unwrap().id());
        self.last_startup = Some(time::Instant::now());

        Ok(())
    }

    pub fn exited(&mut self) -> Option<i32> {
        self.child.as_ref()?;

        match self.child.as_mut().unwrap().try_wait() {
            Ok(Some(status)) => match status.code() {
                Some(code) => Some(code),
                None => {
                    if let Some(signal) = status.signal() {
                        log_info!("PID {} terminated by signal {}", self.id().expect("something went very wrong"), signal);
                    } else {
                        log_info!(
                            "PID {} terminated without exit or signal information",
                            self.id().expect("something went very wrong")
                        )
                    }
                    None
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
            name: ("".to_string()),
            child: None,
            conf: &mut conf::proc::ProcessConfig::testconfig(),
            last_startup: None,
            startup_failures: 0,
            runtime_failures: 0,
            state: Mutex::new(ProcessState::Idle),
        };

        assert!(!proc.running())
    }

    #[test]
    fn running_has_id() {
        let proc = Process {
            id: Some(1),
            name: ("".to_string()),
            child: Some(Command::new("/bin/ls").stdout(Stdio::null()).spawn().expect("could not run command")),
            conf: &mut conf::proc::ProcessConfig::testconfig(),
            last_startup: None,
            startup_failures: 0,
            runtime_failures: 0,
            state: Mutex::new(ProcessState::Idle),
        };

        assert!(proc.running())
    }

    #[test]
    fn state() {
        let proc = Process {
            id: Some(1),
            name: ("".to_string()),
            child: None,
            conf: &mut conf::proc::ProcessConfig::testconfig(),
            last_startup: None,
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
            conf: &mut conf::proc::ProcessConfig::testconfig(),
            last_startup: None,
            startup_failures: 0,
            runtime_failures: 0,
            state: Mutex::new(ProcessState::Idle),
        };
        proc.update_state(ProcessState::Running);
        assert_eq!(proc.state(), ProcessState::Running)
    }
}
