use std::time::Instant;

use crate::{
    proc_info, proc_warning,
    run::{proc::Process, statemachine::states::ProcessState},
};

use super::states::{Completed, Failed, HealthCheck, Healthy, Idle, Ready, State, Stopped, WaitingForRetry};

impl State for Idle {
    fn handle(&self, p: &mut Process) -> Option<ProcessState> {
        None
    }
}

impl State for Ready {
    fn handle(&self, p: &mut Process) -> Option<ProcessState> {
        match p.start() {
            Ok(()) => {
                let pid = p.id().expect("id should always be set if the process is running");
                proc_info!(&p.name(), "spawned, PID {}", pid);
                Some(ProcessState::HealthCheck(Instant::now()))
            }
            Err(err) => {
                proc_warning!(&p.name(), "failed to start: {}", err);
                p.increment_startup_failures();
                Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(Instant::now()))))
            }
        }
    }
}

impl State for HealthCheck {
    fn handle(&self, p: &mut Process) -> Option<ProcessState> {
        if p.healthy(self.started_at()) {
            proc_info!(&p.name(), "has been running for {} seconds, marking as healthy", p.config().starttime());

            return Some(ProcessState::Healthy);
        }

        if let Some(code) = p.exited() {
            if !p.config().exitcodes().contains(&code) {
                proc_warning!(&p.name(), "exited during healthcheck with unexpected code ({})", code);
                return Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(self.started_at()))));
            } else {
                proc_info!(&p.name(), "exited with healthy code ({})", code);
                return Some(ProcessState::Completed);
            }
        }
        None
    }
}

impl State for Healthy {
    fn handle(&self, p: &mut Process) -> Option<ProcessState> {
        if let Some(code) = p.exited() {
            if !p.config().exitcodes().contains(&code) {
                proc_warning!(&p.name(), "exited with unexpected code ({})", code);
                return Some(ProcessState::Failed(Box::new(ProcessState::Healthy)));
            } else {
                proc_info!(&p.name(), "exited with healthy code ({})", code);
                return Some(ProcessState::Completed);
            }
        }
        None
    }
}

pub fn failed_runtime(p: &mut Process) -> Option<ProcessState> {
    match p.config().autorestart().mode() {
        "always" => Some(ProcessState::HealthCheck(Instant::now())),
        "on-failure" => {
            let max_retries = p.config().autorestart().max_retries();

            if p.runtime_failures() == max_retries {
                proc_warning!(&p.name(), "exited unexpectedly {} times, giving up", p.runtime_failures());
                return Some(ProcessState::Stopped);
            }

            let rem_attempts = max_retries - p.runtime_failures();
            let backoff = p.config().backoff();
            proc_warning!(&p.name(), "retrying in {} second(s) ({} attempt(s) left)", backoff, rem_attempts);

            p.increment_runtime_failures();
            Some(ProcessState::WaitingForRetry(p.retry_at()))
        }
        _ => None,
    }
}

pub fn failed_healthcheck(p: &mut Process) -> Option<ProcessState> {
    if p.startup_failures() == p.config().startretries() {
        proc_warning!(&p.name(), "reached max startretries, giving up");
        Some(ProcessState::Stopped)
    } else {
        proc_warning!(&p.name(), "restarting in {} seconds", p.config().backoff());
        p.increment_startup_failures();
        Some(ProcessState::WaitingForRetry(p.retry_at()))
    }
}

impl State for Failed {
    fn handle(&self, p: &mut Process) -> Option<ProcessState> {
        let prev_state = self.prev_state();
        assert!(matches!(prev_state, ProcessState::HealthCheck(_)) || prev_state == &ProcessState::Healthy);

        match prev_state {
            ProcessState::Healthy => failed_runtime(p),
            ProcessState::HealthCheck(_) => failed_healthcheck(p),
            _ => None,
        }
    }
}

impl State for WaitingForRetry {
    fn handle(&self, p: &mut Process) -> Option<ProcessState> {
        if self.retry_at() > Instant::now() {
            return None;
        }

        match p.start() {
            Ok(()) => {
                proc_info!(&p.name(), "spawned, PID {}", p.id().expect("if the process started, its id should be set"));
                Some(ProcessState::HealthCheck(Instant::now()))
            }
            Err(err) => {
                proc_warning!(&p.name(), "failed to start: {}", err);
                Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(Instant::now()))))
            }
        }
    }
}

impl State for Completed {
    fn handle(&self, p: &mut Process) -> Option<ProcessState> {
        if p.config().autorestart().mode() != "always" {
            return None;
        }

        match p.start() {
            Ok(()) => {
                proc_info!(p.name(), "spawned, PID {}", p.id().expect("if the process started, its id should be set"));
                Some(ProcessState::HealthCheck(Instant::now()))
            }
            Err(err) => {
                proc_warning!(p.name(), "failed to start: {}", err);
                Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(Instant::now()))))
            }
        }
    }
}

impl State for Stopped {
    fn handle(&self, _p: &mut Process) -> Option<ProcessState> {
        None
    }
}
