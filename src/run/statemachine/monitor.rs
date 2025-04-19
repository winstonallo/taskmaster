use std::time::Instant;

use crate::{
    proc_info, proc_warning,
    run::{proc::Process, statemachine::healthcheck::HealthCheckEventType},
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

pub fn monitor_healthcheck(started_at: &Instant, p: &mut Process) -> Option<ProcessState> {
    if let Some(code) = p.exited() {
        if !p.config().exitcodes().contains(&code) {
            proc_warning!(&p, "exited while starting with unexpected code ({})", code);
            return Some(ProcessState::Failed(Box::new(ProcessState::Starting(*started_at))));
        } else {
            proc_info!(&p, "exited while starting with healthy code ({})", code);
            return Some(ProcessState::Completed);
        }
    }

    if let Some(receiver) = p.healthcheck_receiver() {
        match receiver.try_recv() {
            Ok(result) => {
                p.clear_healthcheck();
                match result {
                    HealthCheckEventType::Passed => {
                        proc_info!(&p, "healthcheck successful");
                        Some(ProcessState::Healthy)
                    }
                    HealthCheckEventType::Failed(reason) => {
                        proc_warning!(&p, "healthcheck failed: {}", reason);
                        p.increment_healthcheck_failures();
                        Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(*started_at))))
                    }
                    HealthCheckEventType::TimeOut => {
                        proc_warning!(&p, "healthcheck timed out");
                        p.increment_healthcheck_failures();
                        Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(*started_at))))
                    }
                }
            }
            Err(e) => match e {
                tokio::sync::oneshot::error::TryRecvError::Empty => None,
                tokio::sync::oneshot::error::TryRecvError::Closed => {
                    proc_warning!(&p, "healthcheck channel closed unexpectedly");
                    p.clear_healthcheck();
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
    if p.passed_starttime(*started_at) {
        proc_info!(&p, "has been running for {} seconds, marking as healthy", p.config().starttime());

        return Some(ProcessState::Healthy);
    }

    if let Some(code) = p.exited() {
        if !p.config().exitcodes().contains(&code) {
            proc_warning!(&p, "exited while starting with unexpected code ({})", code);
            return Some(ProcessState::Failed(Box::new(ProcessState::Starting(*started_at))));
        } else {
            proc_info!(&p, "exited while starting with healthy code ({})", code);
            return Some(ProcessState::Completed);
        }
    }
    None
}

pub fn monitor_healthy(p: &mut Process) -> Option<ProcessState> {
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

pub fn failed_runtime(p: &mut Process) -> Option<ProcessState> {
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
    let healthcheck = p
        .config()
        .healthcheck()
        .as_ref()
        .expect("this function should never be called on a process which does not have a healthcheck");

    if p.healthcheck_failures() == healthcheck.retries() {
        proc_warning!(p, "not healthy after {} tries, killing", healthcheck.retries());
        Some(ProcessState::Stopped)
    } else {
        proc_info!(p, "retrying healthcheck in {} seconds", healthcheck.backoff());
        Some(ProcessState::WaitingForRetry(p.retry_at()))
    }
}

pub fn failed_starting(p: &mut Process) -> Option<ProcessState> {
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
            ProcessState::Healthy => failed_runtime(p),
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
            if p.healthcheck().is_some() {
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
