use std::time::{Duration, Instant};

use crate::{
    log_error, log_info,
    run::proc::{Process, ProcessState},
};

fn idle(proc: &mut Process) {
    if !proc.config().autostart() {
        return;
    }

    match proc.start() {
        Ok(()) => {
            log_info!(
                "spawned process '{}', PID {}",
                proc.name(),
                proc.id().expect("if the process started, its id should be set")
            );
            proc.update_state(ProcessState::HealthCheck(Instant::now()))
        }
        Err(err) => {
            log_info!("failed to start process {}: {}", proc.name(), err);
            proc.update_state(ProcessState::Failed(Box::new(ProcessState::HealthCheck(Instant::now()))));
        }
    }
}

fn healthcheck(proc: &mut Process, started_at: Instant) {
    if let Some(code) = proc.exited() {
        if !proc.config().exitcodes().contains(&code) {
            log_info!("process '{}' exited during healthcheck with unexpected code: {}", proc.name(), code);
            proc.update_state(ProcessState::Failed(Box::new(ProcessState::HealthCheck(started_at))));
        } else {
            log_info!("process '{}' exited with healthy exit code, marking as completed", proc.name());
            proc.update_state(ProcessState::Completed);
        }
    } else if Instant::now().duration_since(started_at).as_secs() >= proc.config().starttime() as u64 {
        log_info!(
            "process '{}' has been running since {} seconds, marking as healthy",
            proc.name(),
            proc.config().starttime()
        );
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
                let max_retries = proc
                    .config()
                    .autorestart()
                    .max_retries()
                    .expect("max retries should always be set if mode is 'on-failure'");

                if proc.runtime_failures() == max_retries {
                    log_info!("process '{}' exited unexpectedly {} times, giving up", proc.name(), proc.runtime_failures());
                    proc.update_state(ProcessState::Stopped);
                } else {
                    proc.increment_runtime_failures();
                    proc.update_state(ProcessState::WaitingForRetry(
                        Instant::now() + Duration::from_secs(proc.config().backoff() as u64),
                    ));
                    log_info!(
                        "process '{}' exited unexpectedly, retrying in {} second{} ({} {} left)",
                        proc.name(),
                        proc.config().backoff(),
                        if proc.config().backoff() == 1 { "" } else { "s" },
                        max_retries - proc.runtime_failures(),
                        if max_retries - proc.runtime_failures() <= 1 { "try" } else { "tries" }
                    );
                }
            }
            _ => {}
        },
        ProcessState::HealthCheck(_) => {
            if proc.startup_failures() == proc.config().startretries() {
                log_info!("reached max startretries for process '{}', giving up", proc.name());
                proc.update_state(ProcessState::Stopped);
            } else {
                proc.increment_startup_failures();
                proc.update_state(ProcessState::WaitingForRetry(
                    Instant::now() + Duration::from_secs(proc.config().backoff() as u64),
                ));
                log_info!("restarting process '{}' in {} seconds..", proc.name(), proc.config().backoff());
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
        Ok(()) => {
            log_info!(
                "spawned process '{}', PID {}",
                proc.name(),
                proc.id().expect("if the process started, its id should be set")
            );
            proc.update_state(ProcessState::HealthCheck(Instant::now()))
        }
        Err(err) => {
            log_info!("failed to start process {}: {}", proc.name(), err);
            proc.update_state(ProcessState::Failed(Box::new(ProcessState::HealthCheck(Instant::now()))));
        }
    }
}

fn completed(proc: &mut Process) {
    if proc.config().autorestart().mode() != "always" {
        return;
    }

    match proc.start() {
        Ok(()) => {
            log_info!(
                "spawned process '{}', PID {}",
                proc.name(),
                proc.id().expect("if the process started, its id should be set")
            );
            proc.update_state(ProcessState::HealthCheck(Instant::now()))
        }
        Err(err) => {
            log_error!("failed to start process {}: {}", proc.name(), err);
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
