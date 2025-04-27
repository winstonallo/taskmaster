use std::time::Instant;

use crate::run::proc::Process;

use super::states::ProcessState;

const RETAIN_DESIRED_STATE: bool = false;
const REMOVE_DESIRED_STATE: bool = true;

pub fn desire_idle(proc: &mut Process) -> (Option<ProcessState>, bool) {
    use ProcessState::*;
    match proc.state().clone() {
        Idle => (None, REMOVE_DESIRED_STATE),
        Healthy | HealthCheck(_) | Failed(_) => {
            let _ = proc.kill_gracefully();
            (Some(Stopping(Instant::now())), RETAIN_DESIRED_STATE)
        }
        Stopping(_) => (None, RETAIN_DESIRED_STATE),
        _ => (Some(Idle), REMOVE_DESIRED_STATE),
    }
}

pub fn desire_stopped(proc: &mut Process) -> (Option<ProcessState>, bool) {
    use ProcessState::*;
    match proc.state().clone() {
        Healthy | HealthCheck(_) | Failed(_) => {
            let _ = proc.kill_gracefully();
            (Some(Stopping(Instant::now())), RETAIN_DESIRED_STATE)
        }
        Stopping(_) => (None, RETAIN_DESIRED_STATE),
        Stopped => (Some(Stopped), REMOVE_DESIRED_STATE),
        _ => (Some(Stopped), REMOVE_DESIRED_STATE),
    }
}

pub fn desire_ready(proc: &mut Process) -> (Option<ProcessState>, bool) {
    use ProcessState::*;
    match proc.state().clone() {
        Idle => (Some(Ready), REMOVE_DESIRED_STATE),
        Healthy | HealthCheck(_) => {
            let _ = proc.kill_gracefully();
            (Some(Stopping(Instant::now())), RETAIN_DESIRED_STATE)
        }
        Stopping(_) => (None, RETAIN_DESIRED_STATE),
        _ => (Some(Ready), REMOVE_DESIRED_STATE),
    }
}
pub fn desire_healthy(proc: &mut Process) -> (Option<ProcessState>, bool) {
    use ProcessState::*;
    match proc.state().clone() {
        Idle => (Some(Ready), REMOVE_DESIRED_STATE),
        Ready | HealthCheck(_) | Healthy => (None, REMOVE_DESIRED_STATE),
        Stopping(_) => (None, RETAIN_DESIRED_STATE),
        _ => (Some(Ready), REMOVE_DESIRED_STATE),
    }
}
