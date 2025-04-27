use std::{
    fmt::{Debug, Display},
    time::{self, Instant},
};

use crate::run::{
    proc::Process,
    statemachine::{
        desired::{desire_healthy, desire_idle, desire_ready, desire_stopped},
        monitor::{
            monitor_completed, monitor_failed, monitor_healthcheck, monitor_healthy, monitor_idle, monitor_ready, monitor_stopped, monitor_stopping,
            monitor_waiting_for_retry,
        },
    },
};

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
    Stopping(time::Instant),
    Stopped,
}

impl Display for ProcessState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let now = Instant::now();
        match self {
            ProcessState::Idle => write!(f, "idle"),
            ProcessState::Ready => write!(f, "ready"),
            ProcessState::HealthCheck(started_at) => write!(f, "in healthcheck since {} seconds", now.duration_since(*started_at).as_secs()),
            ProcessState::Healthy => write!(f, "healthy"),
            ProcessState::Failed(prev_state) => write!(f, "failed while in state: {}", *prev_state),
            ProcessState::WaitingForRetry(retry_at) => write!(f, "retrying in {} seconds", retry_at.duration_since(now).as_secs()),
            ProcessState::Completed => write!(f, "completed successfully"),
            ProcessState::Stopping(_) => write!(f, "stopping"),
            ProcessState::Stopped => write!(f, "stopped"),
        }
    }
}

impl ProcessState {
    pub fn monitor(&mut self, proc: &mut Process) -> Option<ProcessState> {
        use ProcessState::*;
        match self {
            Idle => monitor_idle(),
            Ready => monitor_ready(proc),
            HealthCheck(started_at) => monitor_healthcheck(started_at, proc),
            Healthy => monitor_healthy(proc),
            Failed(_process_state) => monitor_failed(proc),
            WaitingForRetry(retry_at) => monitor_waiting_for_retry(retry_at, proc),
            Completed => monitor_completed(proc),
            Stopping(killed_at) => monitor_stopping(*killed_at, proc),
            Stopped => monitor_stopped(proc),
        }
    }

    pub fn desire(&mut self, proc: &mut Process) -> Option<ProcessState> {
        let desired_state = match proc.desired_states().front() {
            Some(d_s) => d_s.clone(),
            None => return None,
        };

        use ProcessState::*;
        let (o, remove_desired_state) = match desired_state {
            Idle => desire_idle(proc),
            Stopping(_) | Stopped => desire_stopped(proc),
            Ready => desire_ready(proc),
            HealthCheck(_) | Healthy => desire_healthy(proc),
            Completed => panic!("target ProcessState `Completed` doesn't make sense"),
            Failed(_) => panic!("target ProcessState `Failed` doesn't make sense"),
            WaitingForRetry(_) => panic!("target ProcessState `WaitingForRetry` doesn't make sense"),
        };

        if remove_desired_state {
            proc.desired_states_mut().pop_front();
        }

        o
    }
}
