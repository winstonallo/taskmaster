use state::ProcessState;
use transitions::{completed, failed, healthcheck, idle, running, waiting_for_retry};

use super::proc::Process;

pub mod state;

mod transitions;

/// Monitors the state of `proc` based on the rules defined in taskmaster's process
/// state diagram (`assets/statediagram.png`).
pub fn monitor_state(proc: &mut Process) {
    match proc.state() {
        ProcessState::Idle => idle(proc),
        ProcessState::HealthCheck(started_at) => healthcheck(proc, started_at),
        ProcessState::Healthy => running(proc),
        ProcessState::Failed(prev_state) => failed(proc, *prev_state),
        ProcessState::WaitingForRetry(retry_at) => waiting_for_retry(proc, retry_at),
        ProcessState::Completed => completed(proc),
        ProcessState::Stopped => {}
    }
}
