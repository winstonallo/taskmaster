use std::time::Instant;

use crate::{
    log_error, log_info,
    run::{proc::Process, statemachine::state::ProcessState},
};

use super::state::{Completed, Failed, HealthCheck, Healthy, Idle, State, Stopped, WaitingForRetry};

impl State for Idle {
    fn handle(&self, proc: &mut Process) -> Option<ProcessState> {
        if !proc.config().autostart() {
            return None;
        }

        match proc.start() {
            Ok(()) => {
                let pid = proc.id().expect("if the process started, its id shuld be set");
                log_info!("spawned process '{}', PID {}", proc.name(), pid);
                Some(ProcessState::HealthCheck(Instant::now()))
            }
            Err(err) => {
                log_info!("failed to start process {}: {}", proc.name(), err);
                Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(Instant::now()))))
            }
        }
    }
}

impl State for HealthCheck {
    fn handle(&self, proc: &mut Process) -> Option<ProcessState> {
        if proc.healthy(self.started_at()) {
            log_info!(
                "process '{}' has been running for {} seconds, marking as healthy",
                proc.name(),
                proc.config().starttime()
            );

            proc.update_state(ProcessState::Healthy);
            return None;
        }

        if let Some(code) = proc.exited() {
            if !proc.config().exitcodes().contains(&code) {
                log_info!("process '{}' exited during healthcheck with unexpected code: {}", proc.name(), code);
                return Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(self.started_at()))));
            } else {
                log_info!("process '{}' exited with healthy exit code, marking as completed", proc.name());
                return Some(ProcessState::Completed);
            }
        }
        None
    }
}

impl State for Healthy {
    fn handle(&self, proc: &mut Process) -> Option<ProcessState> {
        if let Some(code) = proc.exited() {
            if !proc.config().exitcodes().contains(&code) {
                return Some(ProcessState::Failed(Box::new(ProcessState::Healthy)));
            } else {
                return Some(ProcessState::Completed);
            }
        }
        None
    }
}

pub fn failed_runtime(proc: &mut Process) -> Option<ProcessState> {
    match proc.config().autorestart().mode() {
        "always" => Some(ProcessState::HealthCheck(Instant::now())),
        "on-failure" => {
            let max_retries = proc.config().autorestart().max_retries();

            if proc.runtime_failures() == max_retries {
                log_info!("process '{}' exited unexpectedly {} times, giving up", proc.name(), proc.runtime_failures());
                return Some(ProcessState::Stopped);
            }

            log_info!(
                "process '{}' exited unexpectedly, retrying in {} second(s) ({} attempt(s) left)",
                proc.name(),
                proc.config().backoff(),
                max_retries - proc.runtime_failures(),
            );

            proc.increment_runtime_failures();
            Some(ProcessState::WaitingForRetry(proc.retry_at()))
        }
        _ => None,
    }
}

pub fn failed_healthcheck(proc: &mut Process) -> Option<ProcessState> {
    if proc.startup_failures() == proc.config().startretries() {
        log_info!("reached max startretries for process '{}', giving up", proc.name());

        Some(ProcessState::Stopped)
    } else {
        log_info!("restarting process '{}' in {} seconds..", proc.name(), proc.config().backoff());

        proc.increment_startup_failures();
        Some(ProcessState::WaitingForRetry(proc.retry_at()))
    }
}

impl State for Failed {
    fn handle(&self, proc: &mut Process) -> Option<ProcessState> {
        let prev_state = self.prev_state();
        assert!(matches!(prev_state, ProcessState::HealthCheck(_)) || prev_state == &ProcessState::Healthy);

        match prev_state {
            ProcessState::Healthy => failed_runtime(proc),
            ProcessState::HealthCheck(_) => failed_healthcheck(proc),
            _ => None,
        }
    }
}

impl State for WaitingForRetry {
    fn handle(&self, proc: &mut Process) -> Option<ProcessState> {
        if self.retry_at() > Instant::now() {
            return None;
        }

        match proc.start() {
            Ok(()) => {
                log_info!(
                    "spawned process '{}', PID {}",
                    proc.name(),
                    proc.id().expect("if the process started, its id should be set")
                );
                Some(ProcessState::HealthCheck(Instant::now()))
            }
            Err(err) => {
                log_info!("failed to start process {}: {}", proc.name(), err);
                Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(Instant::now()))))
            }
        }
    }
}

impl State for Completed {
    fn handle(&self, proc: &mut Process) -> Option<ProcessState> {
        if proc.config().autorestart().mode() != "always" {
            return None;
        }

        match proc.start() {
            Ok(()) => {
                log_info!(
                    "spawned process '{}', PID {}",
                    proc.name(),
                    proc.id().expect("if the process started, its id should be set")
                );
                Some(ProcessState::HealthCheck(Instant::now()))
            }
            Err(err) => {
                log_error!("failed to start process {}: {}", proc.name(), err);
                Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(Instant::now()))))
            }
        }
    }
}

impl State for Stopped {
    fn handle(&self, _proc: &mut Process) -> Option<ProcessState> {
        None
    }
}
