use std::time::Instant;

use crate::{
    proc_error, proc_info, proc_warning,
    run::{
        proc::{Process, ProcessError},
        statemachine::healthcheck::HealthCheckEvent,
    },
};

use super::states::ProcessState;

pub fn monitor_idle() -> Option<ProcessState> {
    None
}

pub async fn monitor_ready(p: &mut Process) -> Option<ProcessState> {
    match p.start().await {
        Ok(()) => {
            let pid = p.id().expect("id should always be set if the process is running");
            proc_info!(&p, "spawned",; pid = pid);
            Some(ProcessState::HealthCheck(Instant::now()))
        }
        Err(err) => {
            proc_error!(&p, "failed to start: {err}");
            p.increment_healthcheck_failures();
            Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(Instant::now()))))
        }
    }
}

/// Check whether `p` exited and return the appropriate state based on its
/// exit code.
///
/// Returns `None` if `p` is running.
fn exited_state(p: &mut Process) -> Option<ProcessState> {
    assert!(matches!(p.state(), ProcessState::HealthCheck(_) | ProcessState::Healthy));

    match p.exited() {
        Ok(code) => {
            if p.config().exitcodes().contains(&code) {
                proc_info!(&p, "exited with healthy code",; code = code);
                Some(ProcessState::Completed)
            } else {
                proc_warning!(&p, "exited with unexpected code",; code = code);
                Some(ProcessState::Failed(Box::new(p.state())))
            }
        }
        Err(e) => match e {
            ProcessError::NoChildProcess | ProcessError::NoExitInformation => Some(ProcessState::Stopped),
            _ => None,
        },
    }
}

fn healthcheck_command(started_at: &Instant, p: &mut Process) -> Option<ProcessState> {
    assert!(p.has_command_healthcheck(), "a process without configured healthcheck should never be used here");

    if let Some(exited_state) = exited_state(p) {
        p.healthcheck_mut().clear();
        return Some(exited_state);
    }

    let receiver = match p.healthcheck_mut().receiver() {
        Some(receiver) => receiver,
        None => {
            proc_info!(p, "starting healthcheck",; cmd = p.healthcheck().cmd(), args = p.healthcheck().args());
            p.start_healthcheck();
            return None;
        }
    };

    match receiver.try_recv() {
        Ok(result) => {
            p.healthcheck_mut().clear();
            match result {
                HealthCheckEvent::Passed => {
                    proc_info!(&p, "healthcheck successful");
                    Some(ProcessState::Healthy)
                }
                HealthCheckEvent::Failed(reason) => {
                    proc_warning!(&p, "healthcheck failed: {reason}");
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
}

fn healthcheck_starttime(started_at: &Instant, p: &mut Process) -> Option<ProcessState> {
    assert!(!p.has_command_healthcheck(), "a process with a configured healthcheck should never be used here");

    if let Some(exited_state) = exited_state(p) {
        return Some(exited_state);
    }

    if p.passed_starttime(*started_at) {
        proc_info!(&p, "has been running for {} seconds, marking as healthy", p.healthcheck().starttime());

        return Some(ProcessState::Healthy);
    }

    None
}

pub fn monitor_healthcheck(started_at: &Instant, p: &mut Process) -> Option<ProcessState> {
    match p.has_command_healthcheck() {
        true => healthcheck_command(started_at, p),
        false => healthcheck_starttime(started_at, p),
    }
}

pub fn monitor_healthy(p: &mut Process) -> Option<ProcessState> {
    exited_state(p)
}

pub fn failed_healthy(p: &mut Process) -> Option<ProcessState> {
    match p.config().autorestart().mode() {
        "always" => Some(ProcessState::HealthCheck(Instant::now())),
        "on-failure" => {
            let max_retries = p.config().autorestart().max_retries();

            if p.runtime_failures() == max_retries as usize {
                proc_warning!(&p, "exited unexpectedly {} times, giving up", p.runtime_failures());
                return Some(ProcessState::Stopped);
            }

            let rem_attempts = max_retries as usize - p.runtime_failures();
            let backoff = p.healthcheck().backoff();
            proc_warning!(&p, "retrying in {} second(s) ({} attempt(s) left)", backoff, rem_attempts);

            p.increment_runtime_failures();
            Some(ProcessState::WaitingForRetry(p.retry_at()))
        }
        _ => None,
    }
}

pub fn failed_healthcheck(p: &mut Process) -> Option<ProcessState> {
    p.increment_healthcheck_failures();

    if p.healthcheck_failures() > p.healthcheck().retries() {
        p.push_desired_state(ProcessState::Stopped);

        proc_warning!(p, "not healthy after {} attempts, giving up", p.healthcheck().retries());
        None
    } else {
        proc_info!(p, "retrying healthcheck in {} seconds", p.healthcheck().backoff());
        Some(ProcessState::WaitingForRetry(p.retry_at()))
    }
}

pub fn monitor_failed(p: &mut Process) -> Option<ProcessState> {
    if let ProcessState::Failed(prev_state) = p.state().clone() {
        assert!(matches!(*prev_state, ProcessState::HealthCheck(_) | ProcessState::Healthy));

        match *prev_state {
            ProcessState::Healthy => failed_healthy(p),
            ProcessState::HealthCheck(_) => failed_healthcheck(p),
            _ => None,
        }
    } else {
        None
    }
}

pub async fn monitor_waiting_for_retry(retry_at: &Instant, p: &mut Process) -> Option<ProcessState> {
    if retry_at > &Instant::now() {
        return None;
    }

    match p.start().await {
        Ok(()) => {
            proc_info!(&p, "spawned, PID {}", p.id().expect("if the process started, its id should be set"));
            Some(ProcessState::HealthCheck(Instant::now()))
        }
        Err(e) => match e {
            ProcessError::AlreadyRunning => Some(ProcessState::HealthCheck(Instant::now())),
            _ => {
                proc_warning!(&p, "failed to start: {}", e);
                Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(Instant::now()))))
            }
        },
    }
}

pub async fn monitor_completed(p: &mut Process) -> Option<ProcessState> {
    if p.config().autorestart().mode() != "always" {
        return None;
    }

    match p.start().await {
        Ok(()) => {
            proc_info!(p, "spawned, PID {}", p.id().expect("if the process started, its id should be set"));
            Some(ProcessState::HealthCheck(Instant::now()))
        }
        Err(err) => {
            proc_warning!(p, "failed to start: {}", err);
            Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(Instant::now()))))
        }
    }
}

pub fn monitor_stopping(killed_at: Instant, p: &mut Process) -> Option<ProcessState> {
    match p.exited() {
        Ok(code) => {
            if p.config().exitcodes().contains(&code) {
                proc_info!(&p, "exited with healthy code",; code = code);
                Some(ProcessState::Completed)
            } else {
                proc_warning!(&p, "exited with unexpected code",; code = code);
                Some(ProcessState::Failed(Box::new(p.state())))
            }
        }
        Err(e) => match e {
            ProcessError::NoChildProcess => Some(ProcessState::Stopped),
            ProcessError::NoExitInformation => {
                proc_warning!(&p, "exited, {e}");
                Some(ProcessState::Stopped)
            }
            _ => {
                if killed_at.elapsed().as_secs() >= p.config().stoptime() as u64 {
                    proc_info!(p, "will now be killed forcefully");
                    let _ = p.kill_forcefully();
                    Some(ProcessState::Stopped)
                } else {
                    None
                }
            }
        },
    }
}

pub fn monitor_stopped(p: &mut Process) -> Option<ProcessState> {
    // Clear failures to start again from 0 if the process gets restarted by the shell.
    p.clear_runtime_failures();
    p.clear_healthcheck_failures();

    // When a process is killed, its entry in the process table is kept
    // until the parent either exits or calls wait() on it.
    let _ = p.exited();

    None
}
