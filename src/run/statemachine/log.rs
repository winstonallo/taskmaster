use crate::{log_fatal, log_info, run::proc::Process};

use super::state::ProcessState;

pub fn log_state_transition(from: &ProcessState, to: &ProcessState, proc: &Process) {
    match from {
        ProcessState::Idle => match to {
            ProcessState::HealthCheck(_) => {
                let pid = proc.id().expect("id should always be set if the process is running");
                log_info!("spawned process '{}', PID {}", proc.name(), pid);
            },
            _ => log_fatal!()
        },
        ProcessState::HealthCheck(started_at) => {}
        ProcessState::Healthy => {}
        ProcessState::Failed(prev_state) => {}
        ProcessState::WaitingForRetry(retry_at) => {}
        ProcessState::Stopped => {}
        ProcessState::Completed => {}
    }
}
