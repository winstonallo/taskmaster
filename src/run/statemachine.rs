use states::{Completed, Failed, HealthCheck, Healthy, Idle, ProcessState, Ready, State, Stopped, WaitingForRetry};

pub use super::proc::Process;

pub mod states;
mod transitions;

fn try_update_state<S: State>(proc: &mut Process, handler: S) {
    if let Some(new_state) = handler.handle(proc) {
        proc.update_state(new_state);
    }
}

/// Monitors the state of `proc` based on the rules defined in taskmaster's process
/// state diagram (`assets/statediagram.png`).
pub fn monitor_state(proc: &mut Process) {
    match proc.state() {
        ProcessState::Idle => try_update_state(proc, Idle),
        ProcessState::Ready => try_update_state(proc, Ready),
        ProcessState::HealthCheck(started_at) => try_update_state(proc, HealthCheck::new(started_at)),
        ProcessState::Healthy => try_update_state(proc, Healthy),
        ProcessState::Failed(prev_state) => try_update_state(proc, Failed::new(*prev_state)),
        ProcessState::WaitingForRetry(retry_at) => try_update_state(proc, WaitingForRetry::new(retry_at)),
        ProcessState::Completed => try_update_state(proc, Completed),
        ProcessState::Stopped => try_update_state(proc, Stopped),
    }
}
