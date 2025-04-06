use std::time::Instant;

use crate::{proc_info, proc_warning, run::proc::Process};

use super::states::ProcessState;

pub fn monitor_idle() -> Option<ProcessState> {
    None
}

pub fn monitor_ready(p: &mut Process) -> Option<ProcessState> {
    match p.start() {
        Ok(()) => {
            let pid = p.id().expect("id should always be set if the process is running");
            proc_info!(p, "spawned, PID {}", pid);
            Some(ProcessState::HealthCheck(Instant::now()))
        }
        Err(err) => {
            proc_warning!(p, "failed to start: {}", err);
            p.increment_startup_failures();
            Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(Instant::now()))))
        }
    }
}

pub fn monitor_health_check(started_at: &Instant, p: &mut Process) -> Option<ProcessState> {
    if let Some(code) = p.exited() {
        if !p.config().exitcodes().contains(&code) {
            proc_warning!(p, "exited during healthcheck with unexpected code ({})", code);
            return Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(*started_at))));
        } else {
            proc_info!(p, "exited with healthy code ({})", code);
            return Some(ProcessState::Completed);
        }
    }

    if p.healthy(*started_at) {
        match p.config().healthcheck().command() {
            Some(_) => proc_info!(p, "healthcheck command successful, marking as healthy"),
            None => proc_info!(p, "has been running for {} seconds, marking as healthy", p.config().healthcheck().starttime()),
        }

        return Some(ProcessState::Healthy);
    } else if started_at.elapsed().as_secs() >= p.config().healthcheck().backoff() as u64 {
        if p.healthcheck_failures() > p.config().healthcheck().retries() {
            let _ = p.config().healthcheck().reset();
            proc_warning!(p, "reached max retries, giving up");
            let _ = p.kill_forcefully();
            return Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(*started_at))));
        }
        let _ = p.config().healthcheck().reset();
        proc_warning!(p, "healthcheck failed, retrying in {}s", p.config().healthcheck().backoff());
        p.set_last_healthcheck_attempt(Some(Instant::now()));
        p.increment_healthcheck_failures();
        return Some(ProcessState::HealthCheck(Instant::now()));
    }

    None
}

pub fn monitor_healthy(p: &mut Process) -> Option<ProcessState> {
    if let Some(code) = p.exited() {
        if !p.config().exitcodes().contains(&code) {
            proc_warning!(p, "exited with unexpected code ({})", code);
            return Some(ProcessState::Failed(Box::new(ProcessState::Healthy)));
        } else {
            proc_info!(p, "exited with healthy code ({})", code);
            return Some(ProcessState::Completed);
        }
    }
    None
}

pub fn failed_runtime(p: &mut Process) -> Option<ProcessState> {
    match p.config().autorestart().mode() {
        "always" => Some(ProcessState::HealthCheck(Instant::now())),
        "on-failure" => {
            let max_retries = p.config().autorestart().max_retries();

            if p.runtime_failures() == max_retries {
                proc_warning!(p, "exited unexpectedly {} times, giving up", p.runtime_failures());
                return Some(ProcessState::Stopped);
            }

            let rem_attempts = max_retries - p.runtime_failures();
            let backoff = p.config().backoff();
            proc_warning!(p, "retrying in {} second(s) ({} attempt(s) left)", backoff, rem_attempts);

            p.increment_runtime_failures();
            Some(ProcessState::WaitingForRetry(p.retry_at()))
        }
        _ => None,
    }
}

pub fn failed_healthcheck(p: &mut Process) -> Option<ProcessState> {
    if p.startup_failures() == p.config().healthcheck().retries() {
        proc_warning!(p, "reached max startretries, giving up");
        Some(ProcessState::Stopped)
    } else {
        proc_warning!(p, "restarting in {} seconds", p.config().backoff());
        p.increment_startup_failures();
        Some(ProcessState::WaitingForRetry(p.retry_at()))
    }
}

pub fn monitor_failed(p: &mut Process) -> Option<ProcessState> {
    if let ProcessState::Failed(prev_state) = p.state().clone() {
        assert!(matches!(*prev_state, ProcessState::HealthCheck(_)) || *prev_state == ProcessState::Healthy);

        match *prev_state {
            ProcessState::Healthy => failed_runtime(p),
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
            proc_info!(p, "spawned, PID {}", p.id().expect("if the process started, its id should be set"));
            Some(ProcessState::HealthCheck(Instant::now()))
        }
        Err(err) => {
            proc_warning!(p, "failed to start: {}", err);
            Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(Instant::now()))))
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
            Some(ProcessState::HealthCheck(Instant::now()))
        }
        Err(err) => {
            proc_warning!(p, "failed to start: {}", err);
            Some(ProcessState::Failed(Box::new(ProcessState::HealthCheck(Instant::now()))))
        }
    }
}

pub fn monitor_stopping(_p: &mut Process) -> Option<ProcessState> {
    None
}

pub fn monitor_stopped(_p: &mut Process) -> Option<ProcessState> {
    None
}
