use std::{
    collections::VecDeque, error::Error, ffi::CString, fs::File, os::unix::process::{CommandExt, ExitStatusExt}, process::{Child, Command, ExitStatus, Stdio}, time::{self, Duration, Instant}
};

use crate::{
    conf::{self, proc::ProcessConfig},
    log_error, proc_info,
};
pub use error::ProcessError;
use libc::{gid_t, setgid, setgroups, setuid, umask};

use super::statemachine::{healthcheck::HealthCheckRunner, states::ProcessState};

mod error;
mod tests;

#[allow(unused)]
#[derive(Debug)]
pub struct Process {
    id: Option<u32>,
    name: String,
    child: Option<Child>,
    conf: ProcessConfig,
    healthcheck: HealthCheckRunner,
    runtime_failures: usize,
    state: ProcessState,
    desired_states: VecDeque<ProcessState>,
}

impl Process {
    pub fn from_process_config(conf: conf::proc::ProcessConfig, proc_name: &str) -> Self {
        let is_autostart = conf.autostart();
        let healthcheck = conf.healthcheck().clone();
        Self {
            id: None,
            name: proc_name.to_string(),
            child: None,
            conf,
            healthcheck: HealthCheckRunner::from_healthcheck_config(&healthcheck),
            runtime_failures: 0,
            state: ProcessState::Idle,
            desired_states: match is_autostart {
                true => VecDeque::from([ProcessState::Ready]),
                false => VecDeque::new(),
            },
        }
    }

    pub async fn monitor(&mut self) {
        let new_state = match self.state.clone().monitor(self).await {
            Some(new_state) => new_state,
            None => return,
        };
        self.state = new_state;
    }

    pub fn desire(&mut self) {
        let new_state = match self.state.clone().desire(self) {
            Some(new_state) => new_state,
            None => return,
        };
        self.state = new_state; // desired state
    }
}

#[allow(unused)]
impl Process {
    pub fn state(&self) -> ProcessState {
        self.state.clone()
    }

    pub fn push_desired_state(&mut self, desired_state: ProcessState) {
        self.desired_states.push_back(desired_state);
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

    pub fn desired_states(&self) -> &VecDeque<ProcessState> {
        &self.desired_states
    }

    pub fn desired_states_mut(&mut self) -> &mut VecDeque<ProcessState> {
        &mut self.desired_states
    }

    pub fn runtime_failures(&self) -> usize {
        self.runtime_failures
    }

    pub fn clear_runtime_failures(&mut self) {
        self.runtime_failures = 0;
    }

    pub fn increment_runtime_failures(&mut self) {
        self.runtime_failures = self.runtime_failures.saturating_add(1);
    }

    pub fn healthcheck_failures(&self) -> usize {
        self.healthcheck.failures()
    }

    pub fn increment_healthcheck_failures(&mut self) {
        self.healthcheck.increment_failures();
    }

    pub fn clear_healthcheck_failures(&mut self) {
        self.healthcheck.clear_failures();
    }

    /// Returns `true` if a dynamic healthcheck is configured for this process.
    pub fn has_command_healthcheck(&self) -> bool {
        self.healthcheck.has_command_healthcheck()
    }

    pub fn healthcheck(&self) -> &HealthCheckRunner {
        &self.healthcheck
    }

    pub fn healthcheck_mut(&mut self) -> &mut HealthCheckRunner {
        &mut self.healthcheck
    }

    pub fn retry_at(&self) -> time::Instant {
        Instant::now() + Duration::from_secs(self.healthcheck().backoff() as u64)
    }

    pub fn start_healthcheck(&mut self) {
        self.healthcheck.start();
    }

    pub fn passed_starttime(&self, started_at: time::Instant) -> bool {
        Instant::now().duration_since(started_at).as_secs() >= self.healthcheck.starttime() as u64
    }

    fn get_group_id(group_name: &str) -> Result<u32, String> {
        let c_group = CString::new(group_name).map_err(|e| format!("{e}"))?;
    
        unsafe {
            let grp_ptr = libc::getgrnam(c_group.as_ptr());
            if grp_ptr.is_null() {
                Err(format!("group '{group_name}' not found"))
            } else {
                Ok((*grp_ptr).gr_gid)
            }
        }
    }

    async fn spawn(&self) -> Result<Child, Box<dyn Error + Send + Sync>> {
        let stdout_file = File::create(self.conf.stdout()).map_err(|err| ProcessError::Internal(err.to_string()))?;
        let stderr_file = File::create(self.conf.stderr()).map_err(|err| ProcessError::Internal(err.to_string()))?;

        let cmd_path = self.conf.cmd().path().to_owned();
        let args = self.conf.args().to_owned();
        let working_dir = self.conf.workingdir().path();
        let stop_signals = self.conf.stopsignals().to_owned();
        let umask_val = self.conf.umask();
        let uid = if let Some(user) = self.conf.user() {
            Some(Process::get_group_id(user).map_err(|e| e.to_string())?)
        } else {
            None
        };
        let gid = if let Some(user) = self.conf.user() {
            Some(Process::get_group_id(user).map_err(|e| e.to_string())?)
        } else {
            None
        };


        let mut child = unsafe {
            Command::new(cmd_path)
                .args(args)
                .envs(self.conf.env().clone())
                .stdin(Stdio::piped())
                .stdout(stdout_file)
                .stderr(stderr_file)
                .pre_exec(move || {
                    if let Some(_) = uid {
                        let empty: [gid_t; 1] = [gid.unwrap_or(0)];

                        if setgroups(1, empty.as_ptr()) != 0 {
                            return Err(std::io::Error::last_os_error());
                        }
                    }
                    if let Some(gid) = gid {
                        if setgid(gid) != 0 {
                            return Err(std::io::Error::last_os_error());
                        }
                    }

                    if let Some(uid) = uid {
                        if setuid(uid) != 0 {
                            return Err(std::io::Error::last_os_error());
                        }
                    }
                    umask(umask_val);
                    Ok(())
                })
                .current_dir(working_dir)
                .spawn()
                .map_err(|e| Box::<dyn Error + Send + Sync>::from(e.to_string()))
        }?;

        Ok(child)
    }

    pub async fn start(&mut self) -> Result<(), ProcessError> {
        if self.child.is_some() {
            return Err(ProcessError::AlreadyRunning);
        }

        assert_ne!(self.state(), ProcessState::Healthy);

        self.child = match self.spawn().await {
            Ok(child) => Some(child),
            Err(e) => return Err(ProcessError::CouldNotSpawn(e.to_string())),
        };

        self.id = Some(self.child.as_ref().unwrap().id());

        Ok(())
    }

    fn check_signal(&mut self, status: ExitStatus, pid: u32) -> Option<i32> {
        if let Some(signal) = status.signal() {
            proc_info!(self, "terminated",; pid = pid, signal = signal);
        } else if self.state != ProcessState::Stopped {
            proc_info!(self, "terminated without exit or signal information",; pid = pid)
        }
        self.state = ProcessState::Stopped;
        None
    }

    pub fn exited(&mut self) -> Result<i32, ProcessError> {
        if self.child.is_none() {
            return Err(ProcessError::NoChildProcess);
        }

        let pid = self.id().expect("id should always be set if the program is running");

        match self.child.as_mut().unwrap().try_wait() {
            Ok(Some(status)) => match status.code() {
                Some(code) => {
                    self.child = None;
                    Ok(code)
                }
                None => {
                    self.child = None;
                    self.check_signal(status, pid);
                    Err(ProcessError::NoExitInformation)
                }
            },
            Ok(None) => Err(ProcessError::AlreadyRunning),
            Err(err) => {
                log_error!("could not get status for PID {}: {}", self.id().expect("something went very wrong"), err);
                Err(ProcessError::Internal("could not get exit status for process".to_string()))
            }
        }
    }

    pub fn kill_gracefully(&mut self) -> Result<(), &str> {
        use ProcessState::*;
        match self.state() {
            HealthCheck(_) | Healthy | Failed(_) => {}
            _ => return Err("process not running"),
        }

        let child = match &self.child {
            Some(c) => c,
            None => return Err("child is None"),
        };

        unsafe {
            libc::kill(
                child.id() as i32,
                self.config()
                    .stopsignals()
                    .first()
                    .expect("something went terribly wrong")
                    .signal(),
            );
        }
        proc_info!(self, "shutting down, PID {} gracefully", child.id());

        Ok(())
    }

    pub fn kill_forcefully(&mut self) -> Result<(), &str> {
        use ProcessState::*;
        match self.state() {
            HealthCheck(_) | Healthy | Stopping(_) => {}
            _ => return Err("process not running or in stopping state"),
        }

        let mut child = match self.child.take() {
            Some(c) => c,
            None => return Err("child is None"),
        };

        child.kill();
        proc_info!(self, "killed, PID {}", child.id());
        self.id.take();

        Ok(())
    }
}
