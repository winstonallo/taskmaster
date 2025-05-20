use std::{fmt, time::Instant};

use serde::{Deserialize, Serialize};

use crate::run::{proc::Process, statemachine::states::ProcessState};

#[derive(Serialize, Deserialize, Clone, Debug)]
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

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn state(&self) -> &State {
        &self.state
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum State {
    Idle,
    Ready,
    Starting(u64),
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
            ProcessState::WaitingForRetry(instant) => Self::WaitingForRetry(instant.duration_since(Instant::now()).as_secs()),
            ProcessState::Completed => Self::Completed,
            ProcessState::Stopping(instant) => Self::Stopping(instant.elapsed().as_secs()),
            ProcessState::Stopped => Self::Stopped,
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use State::*;
        let s = match self {
            Idle => "idle".to_owned(),
            Ready => "ready".to_owned(),
            Starting(s) => format!("starting since {s} seconds"),
            HealthCheck(s) => format!("healthcheck since {s} seconds"),
            Healthy => "healthy".to_owned(),
            Failed => "failed".to_owned(),
            WaitingForRetry(s) => format!("waiting for retry - {s} seconds left"),
            Completed => "completed".to_owned(),
            Stopping(s) => format!("stopping since {s} seconds"),
            Stopped => "stopped".to_owned(),
        };

        write!(f, "{s}")
    }
}
