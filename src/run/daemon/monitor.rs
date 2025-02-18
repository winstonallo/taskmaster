use std::time::{Duration, Instant};

use crate::run::proc::{Process, ProcessState};

fn idle(proc: &mut Process) {
    if !proc.config().autostart() {
        return;
    }

    match proc.start() {
        Ok(()) => proc.update_state(ProcessState::HealthCheck(Instant::now())),
        Err(err) => {
            eprintln!("failed to start process {}: {}", proc.name(), err);
            proc.update_state(ProcessState::Failed(Box::new(ProcessState::HealthCheck(Instant::now()))));
        }
    }
}

fn healthcheck(proc: &mut Process, started_at: Instant) {
    if let Some(code) = proc.exited() {
        if !proc.config().exitcodes().contains(&code) {
            proc.update_state(ProcessState::Failed(Box::new(ProcessState::HealthCheck(started_at))));
        } else {
            proc.update_state(ProcessState::Completed);
        }
    } else if Instant::now().duration_since(started_at).as_secs() >= proc.config().starttime() as u64 {
        proc.update_state(ProcessState::Running);
    }
}

fn running(proc: &mut Process) {
    if let Some(code) = proc.exited() {
        if !proc.config().exitcodes().contains(&code) {
            proc.update_state(ProcessState::Failed(Box::new(ProcessState::Running)));
        } else {
            proc.update_state(ProcessState::Completed);
        }
    }
}

fn failed(proc: &mut Process, prev_state: ProcessState) {
    match prev_state {
        ProcessState::Running => match proc.config().autorestart().mode() {
            "always" => proc.update_state(ProcessState::HealthCheck(Instant::now())),
            "on-failure" => {
                if proc.runtime_failures() == proc.config().autorestart().max_retries().expect("something went very wrong") {
                    proc.update_state(ProcessState::Stopped);
                } else {
                    proc.increment_runtime_failures();
                    proc.update_state(ProcessState::WaitingForRetry(
                        Instant::now() + Duration::from_secs(proc.config().backoff() as u64),
                    ));
                }
            }
            _ => {}
        },
        ProcessState::HealthCheck(_) => {
            if proc.startup_failures() == proc.config().startretries() {
                proc.update_state(ProcessState::Stopped);
            } else {
                proc.update_state(ProcessState::WaitingForRetry(
                    Instant::now() + Duration::from_secs(proc.config().backoff() as u64),
                ));
            }
        }
        _ => {}
    }
}

fn waiting_for_retry(proc: &mut Process, retry_at: Instant) {
    if retry_at > Instant::now() {
        return;
    }

    match proc.start() {
        Ok(()) => proc.update_state(ProcessState::HealthCheck(Instant::now())),
        Err(err) => {
            eprintln!("failed to start process {}: {}", proc.name(), err);
            proc.update_state(ProcessState::Failed(Box::new(ProcessState::HealthCheck(Instant::now()))));
        }
    }
}

fn completed(proc: &mut Process) {
    if proc.config().autorestart().mode() != "always" {
        return;
    }

    match proc.start() {
        Ok(()) => proc.update_state(ProcessState::HealthCheck(Instant::now())),
        Err(err) => {
            eprintln!("failed to start process {}: {}", proc.name(), err);
            proc.update_state(ProcessState::Failed(Box::new(ProcessState::HealthCheck(Instant::now()))));
        }
    }
}

pub fn monitor_state(proc: &mut Process) {
    match proc.state() {
        ProcessState::Idle => idle(proc),
        ProcessState::HealthCheck(started_at) => healthcheck(proc, started_at),
        ProcessState::Running => running(proc),
        ProcessState::Failed(prev_state) => failed(proc, *prev_state),
        ProcessState::WaitingForRetry(retry_at) => waiting_for_retry(proc, retry_at),
        ProcessState::Completed => completed(proc),
        ProcessState::Stopped => {}
    }
}
