use std::time::Instant;

use crate::run::{
    self,
    proc::{Process, ProcessState},
};

fn idle(proc: &mut Process) {
    if proc.config().autostart() {
        // start process
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

fn failed(proc: &mut Process) {}

pub fn monitor_state(proc: &mut Process) {
    match proc.state() {
        ProcessState::Idle => idle(proc),
        ProcessState::HealthCheck(started_at) => healthcheck(proc, started_at),
        ProcessState::Running => running(proc),
        ProcessState::Failed(previous_state) => {}
        ProcessState::WaitingForRetry(retry_at) => {}
        ProcessState::Completed => {}
    }
}
