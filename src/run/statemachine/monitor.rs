use std::time::Instant;

use crate::{
    proc_info, proc_warning,
    run::{proc::Process, statemachine::healthcheck::HealthCheckEvent},
};

use super::states::ProcessState;

pub fn monitor_idle() -> Option<ProcessState> {
    None
}

pub fn monitor_ready(p: &mut Process) -> Option<ProcessState> {
    match p.start() {
        Ok(()) => {
            let pid = p.id().expect("id should always be set if the process is running");
            proc_info!(&p, "spawned, PID {}", pid);
            if p.config().healthcheck().is_some() {
                Some(ProcessState::HealthCheck(Instant::now()))
            } else {
                Some(ProcessState::Starting(Instant::now()))
            }
        }
        Err(err) => {
            proc_warning!(&p, "failed to start: {}", err);
            p.increment_startup_failures();
            Some(ProcessState::Failed(Box::new(ProcessState::Starting(Instant::now()))))
        }
    }
}

/// Check whether a `p` exited and return the appropriate state based on its
/// exit code.
///
/// Returns `None` if `p` is running.
fn exited_state(p: &mut Process) -> Option<ProcessState> {
    assert!(matches!(p.state(), ProcessState::HealthCheck(_) | ProcessState::Starting(_)));

    if let Some(code) = p.exited() {
        if !p.config().exitcodes().contains(&code) {
            proc_warning!(&p, "exited {:?} with unexpected code ({})", p.state(), code);
            return Some(ProcessState::Failed(Box::new(p.state())));
        } else {
            proc_info!(&p, "exited {} with healthy code ({})", p.state(), code);
            return Some(ProcessState::Completed);
        }
    }

    None
}

pub fn monitor_healthcheck(started_at: &Instant, p: &mut Process) -> Option<ProcessState> {
    assert!(p.has_healthcheck(), "a process without configured healthcheck should never be in the `HealthCheck` state");

    if let Some(exited_state) = exited_state(p) {
        return Some(exited_state);
    }

    if let Some(receiver) = p.healthcheck_mut().receiver() {
        match receiver.try_recv() {
            Ok(result) => {
                p.healthcheck_mut().clear();
                match result {
                    HealthCheckEvent::Passed => {
                        proc_info!(&p, "healthcheck successful");
                        Some(ProcessState::Healthy)
                    }
                    HealthCheckEvent::Failed(reason) => {
                        proc_warning!(&p, "healthcheck failed: {}", reason);
                        Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(*started_at))))
                    }
                }
            }
            Err(e) => match e {
                tokio::sync::oneshot::error::TryRecvError::Empty => None,
                tokio::sync::oneshot::error::TryRecvError::Closed => {
                    proc_warning!(&p, "healthcheck channel closed unexpectedly");
                    p.healthcheck_mut().clear();
                    Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(*started_at))))
                }
            },
        }
    } else {
        p.start_healthcheck();
        None
    }
}

pub fn monitor_starting(started_at: &Instant, p: &mut Process) -> Option<ProcessState> {
    assert!(!p.has_healthcheck(), "a process with a configured healthcheck should never be in the `Starting` state");

    if p.passed_starttime(*started_at) {
        proc_info!(&p, "has been running for {} seconds, marking as healthy", p.config().starttime());

        return Some(ProcessState::Healthy);
    }

    exited_state(p)
}

pub fn monitor_healthy(p: &mut Process) -> Option<ProcessState> {
    exited_state(p)
}

pub fn failed_healthy(p: &mut Process) -> Option<ProcessState> {
    match p.config().autorestart().mode() {
        "always" => Some(ProcessState::Starting(Instant::now())),
        "on-failure" => {
            let max_retries = p.config().autorestart().max_retries();

            if p.runtime_failures() == max_retries as usize {
                proc_warning!(&p, "exited unexpectedly {} times, giving up", p.runtime_failures());
                return Some(ProcessState::Stopped);
            }

            let rem_attempts = max_retries as usize - p.runtime_failures();
            let backoff = p.config().backoff();
            proc_warning!(&p, "retrying in {} second(s) ({} attempt(s) left)", backoff, rem_attempts);

            p.increment_runtime_failures();
            Some(ProcessState::WaitingForRetry(p.retry_at()))
        }
        _ => None,
    }
}

pub fn failed_healthcheck(p: &mut Process) -> Option<ProcessState> {
    p.increment_healthcheck_failures();
    if p.healthcheck_failures() == p.healthcheck().retries() {
        proc_warning!(p, "not healthy after {} attempts, giving up", p.healthcheck().retries());
        Some(ProcessState::Stopped)
    } else {
        proc_info!(p, "retrying healthcheck in {} seconds", p.healthcheck().backoff());
        Some(ProcessState::WaitingForRetry(p.retry_at()))
    }
}

pub fn failed_starting(p: &mut Process) -> Option<ProcessState> {
    assert!(!p.has_healthcheck(), "a process with a configured healthcheck should never be in the `Starting` state");

    if p.startup_failures() == p.config().startretries() as usize {
        proc_warning!(&p, "reached max startretries, giving up");
        Some(ProcessState::Stopped)
    } else {
        proc_warning!(&p, "restarting in {} seconds", p.config().backoff());
        p.increment_startup_failures();
        Some(ProcessState::WaitingForRetry(p.retry_at()))
    }
}

pub fn monitor_failed(p: &mut Process) -> Option<ProcessState> {
    if let ProcessState::Failed(prev_state) = p.state().clone() {
        assert!(matches!(*prev_state, ProcessState::HealthCheck(_) | ProcessState::Starting(_) | ProcessState::Healthy));

        match *prev_state {
            ProcessState::Healthy => failed_healthy(p),
            ProcessState::Starting(_) => failed_starting(p),
            ProcessState::HealthCheck(_) => failed_healthcheck(p),
            _ => None,
        }
    } else {
        None
    }
}

pub fn monitor_waiting_for_retry(retry_at: &Instant, p: &mut Process) -> Option<ProcessState> {
    if retry_at > &Instant::now() {
        return None;
    }

    match p.start() {
        Ok(()) => {
            proc_info!(&p, "spawned, PID {}", p.id().expect("if the process started, its id should be set"));
            if p.config().healthcheck().is_some() {
                Some(ProcessState::HealthCheck(Instant::now()))
            } else {
                Some(ProcessState::Starting(Instant::now()))
            }
        }
        Err(err) => {
            proc_warning!(&p, "failed to start: {}", err);
            Some(ProcessState::Failed(Box::new(ProcessState::Starting(Instant::now()))))
        }
    }
}

pub fn monitor_completed(p: &mut Process) -> Option<ProcessState> {
    if p.config().autorestart().mode() != "always" {
        return None;
    }

    match p.start() {
        Ok(()) => {
            proc_info!(p, "spawned, PID {}", p.id().expect("if the process started, its id should be set"));
            if p.has_healthcheck() {
                Some(ProcessState::HealthCheck(Instant::now()))
            } else {
                Some(ProcessState::Starting(Instant::now()))
            }
        }
        Err(err) => {
            proc_warning!(p, "failed to start: {}", err);
            Some(ProcessState::Failed(Box::new(ProcessState::Starting(Instant::now()))))
        }
    }
}

pub fn monitor_stopping(p: &mut Process) -> Option<ProcessState> {
    if let Some(code) = p.exited() {
        if !p.config().exitcodes().contains(&code) {
            proc_warning!(&p, "exited with unexpected code ({})", code);
            return Some(ProcessState::Failed(Box::new(ProcessState::Healthy)));
        } else {
            proc_info!(&p, "exited with healthy code ({})", code);
            return Some(ProcessState::Completed);
        }
    }
    None
}

pub fn monitor_stopped(_p: &mut Process) -> Option<ProcessState> {
    None
}
