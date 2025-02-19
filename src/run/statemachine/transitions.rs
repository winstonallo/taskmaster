use std::time::Instant;

use crate::{
    log_error, log_info,
    run::{proc::Process, statemachine::state::ProcessState},
};

pub fn idle(proc: &mut Process) {
    if !proc.config().autostart() {
        return;
    }

    match proc.start() {
        Ok(()) => {
            let pid = proc.id().expect("if the process started, its id shuld be set");
            log_info!("spawned process '{}', PID {}", proc.name(), pid);
            proc.update_state(ProcessState::HealthCheck(Instant::now()))
        }
        Err(err) => {
            log_info!("failed to start process {}: {}", proc.name(), err);
            proc.update_state(ProcessState::Failed(Box::new(ProcessState::HealthCheck(Instant::now()))));
        }
    }
}

pub fn healthcheck(proc: &mut Process, started_at: Instant) {
    if proc.healthy(started_at) {
        log_info!(
            "process '{}' has been running for {} seconds, marking as healthy",
            proc.name(),
            proc.config().starttime()
        );

        proc.update_state(ProcessState::Healthy);
        return;
    }

    if let Some(code) = proc.exited() {
        if !proc.config().exitcodes().contains(&code) {
            log_info!("process '{}' exited during healthcheck with unexpected code: {}", proc.name(), code);
            proc.update_state(ProcessState::Failed(Box::new(ProcessState::HealthCheck(started_at))));
        } else {
            log_info!("process '{}' exited with healthy exit code, marking as completed", proc.name());
            proc.update_state(ProcessState::Completed);
        }
    }
}

pub fn running(proc: &mut Process) {
    if let Some(code) = proc.exited() {
        if !proc.config().exitcodes().contains(&code) {
            proc.update_state(ProcessState::Failed(Box::new(ProcessState::Healthy)));
        } else {
            proc.update_state(ProcessState::Completed);
        }
    }
}

pub fn failed_runtime(proc: &mut Process) {
    match proc.config().autorestart().mode() {
        "always" => proc.update_state(ProcessState::HealthCheck(Instant::now())),
        "on-failure" => {
            let max_retries = proc.config().autorestart().max_retries();

            if proc.runtime_failures() == max_retries {
                log_info!("process '{}' exited unexpectedly {} times, giving up", proc.name(), proc.runtime_failures());
                proc.update_state(ProcessState::Stopped);
                return;
            }

            log_info!(
                "process '{}' exited unexpectedly, retrying in {} second(s) ({} attempt(s) left)",
                proc.name(),
                proc.config().backoff(),
                max_retries - proc.runtime_failures(),
            );

            proc.increment_runtime_failures();
            proc.update_state(ProcessState::WaitingForRetry(proc.retry_at()));
        }
        _ => {}
    }
}

pub fn failed_healthcheck(proc: &mut Process) {
    if proc.startup_failures() == proc.config().startretries() {
        log_info!("reached max startretries for process '{}', giving up", proc.name());

        proc.update_state(ProcessState::Stopped);
    } else {
        log_info!("restarting process '{}' in {} seconds..", proc.name(), proc.config().backoff());

        proc.increment_startup_failures();
        proc.update_state(ProcessState::WaitingForRetry(proc.retry_at()));
    }
}

pub fn failed(proc: &mut Process, prev_state: ProcessState) {
    assert!(matches!(prev_state, ProcessState::HealthCheck(_)) || prev_state == ProcessState::Healthy);

    match prev_state {
        ProcessState::Healthy => failed_runtime(proc),
        ProcessState::HealthCheck(_) => failed_healthcheck(proc),
        _ => {}
    }
}

pub fn waiting_for_retry(proc: &mut Process, retry_at: Instant) {
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

pub fn completed(proc: &mut Process) {
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
