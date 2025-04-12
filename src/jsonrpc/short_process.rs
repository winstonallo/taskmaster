use serde::{Deserialize, Serialize};

use crate::run::{proc::Process, statemachine::states::ProcessState};

#[derive(Serialize, Deserialize)]
pub struct ShortProcess {
    name: String,
    state: State,
}

impl ShortProcess {
    pub fn from_process(process: &Process) -> Self {
        Self {
            name: process.name().to_owned(),
            state: State::from_process_state(process.state()),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum State {
    Idle,
    Ready,
    HealthCheck(u64),
    Healthy,
    Failed,
    WaitingForRetry(u64),
    Completed,
    Stopping(u64),
    Stopped,
}

impl State {
    pub fn from_process_state(process_state: ProcessState) -> Self {
        match process_state {
            ProcessState::Idle => Self::Idle,
            ProcessState::Ready => Self::Ready,
            ProcessState::HealthCheck(instant) => Self::HealthCheck(instant.elapsed().as_secs()),
            ProcessState::Healthy => Self::Healthy,
            ProcessState::Failed(_) => Self::Failed,
            ProcessState::WaitingForRetry(instant) => Self::WaitingForRetry(instant.elapsed().as_secs()),
            ProcessState::Completed => Self::Completed,
            ProcessState::Stopping(instant) => Self::Stopping(instant.elapsed().as_secs()),
            ProcessState::Stopped => Self::Stopped,
        }
    }
}
