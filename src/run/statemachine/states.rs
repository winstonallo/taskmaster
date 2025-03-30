use std::{
    fmt::{Debug, Display},
    time::{self, Instant},
};

use crate::run::proc::Process;

#[allow(unused)]
#[derive(Clone, Debug, PartialEq)]
pub enum ProcessState {
    Idle,
    Ready,
    HealthCheck(time::Instant),
    Healthy,
    Failed(Box<ProcessState>),
    WaitingForRetry(time::Instant),
    Completed,
    Stopped,
}

impl Display for ProcessState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessState::Idle => write!(f, "idle"),
            ProcessState::Ready => write!(f, "ready"),
            ProcessState::HealthCheck(started_at) => write!(f, "in healthcheck since {} seconds", Instant::now().duration_since(*started_at).as_secs()),
            ProcessState::Healthy => write!(f, "healthy"),
            ProcessState::Failed(prev_state) => write!(f, "failed while in state: {}", *prev_state),
            ProcessState::WaitingForRetry(retry_at) => write!(f, "retrying in {} seconds", retry_at.duration_since(Instant::now()).as_secs()),
            ProcessState::Completed => write!(f, "completed successfully"),
            ProcessState::Stopped => write!(f, "stopped"),
        }
    }
}

/// Trait to be implemented by for the abstraction of state transitions.
pub trait State {
    /// Returns the new `ProcessState` for `proc`, or `None` if no transition
    /// is required.
    fn handle(&self, proc: &mut Process) -> Option<ProcessState>;
}

pub struct Idle;

pub struct Ready;

pub struct HealthCheck {
    started_at: time::Instant,
}

impl HealthCheck {
    pub fn started_at(&self) -> time::Instant {
        self.started_at
    }

    pub fn new(started_at: time::Instant) -> Self {
        Self { started_at }
    }
}

pub struct Healthy;

pub struct Failed {
    prev_state: ProcessState,
}

impl Failed {
    pub fn prev_state(&self) -> &ProcessState {
        &self.prev_state
    }

    pub fn new(prev_state: ProcessState) -> Self {
        Self { prev_state }
    }
}

pub struct WaitingForRetry {
    retry_at: time::Instant,
}

impl WaitingForRetry {
    pub fn retry_at(&self) -> time::Instant {
        self.retry_at
    }

    pub fn new(retry_at: time::Instant) -> Self {
        Self { retry_at }
    }
}

pub struct Completed;

pub struct Stopped;
