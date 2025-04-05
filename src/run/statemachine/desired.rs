use std::time::Instant;

use crate::run::proc::Process;

use super::states::ProcessState;

const RETAIN_DESIRED_STATE: bool = false;
const REMOVE_DESIRED_STATE: bool = true;

pub fn desire_idle(proc: &mut Process) -> (Option<ProcessState>, bool) {
    use ProcessState::*;
    match proc.state().clone() {
        Idle => (None, REMOVE_DESIRED_STATE),
        Healthy | HealthCheck(_) => {
            let _ = proc.kill_gracefully();
            (Some(Stopping(Instant::now())), RETAIN_DESIRED_STATE)
        }
        Stopping(stopped_at) => {
            if proc.config().stoptime() <= stopped_at.elapsed().as_secs() as u8 {
                let _ = proc.kill_forcefully();
                (Some(Idle), RETAIN_DESIRED_STATE)
            } else {
                (None, RETAIN_DESIRED_STATE)
            }
        }
        _ => (Some(Idle), REMOVE_DESIRED_STATE),
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
        Stopping(stopped_at) => {
            if proc.config().stoptime() <= stopped_at.elapsed().as_secs() as u8 {
                let _ = proc.kill_forcefully();
                (Some(Ready), REMOVE_DESIRED_STATE)
            } else {
                (None, RETAIN_DESIRED_STATE)
            }
        }
        _ => (Some(Ready), REMOVE_DESIRED_STATE),
    }
}
pub fn desire_healthy(proc: &mut Process) -> (Option<ProcessState>, bool) {
    use ProcessState::*;
    match proc.state().clone() {
        Idle => (Some(Ready), REMOVE_DESIRED_STATE),
        Ready | HealthCheck(_) | Healthy => (None, REMOVE_DESIRED_STATE),
        Stopping(stopped_at) => {
            if proc.config().stoptime() <= stopped_at.elapsed().as_secs() as u8 {
                let _ = proc.kill_forcefully();
                (Some(Ready), REMOVE_DESIRED_STATE)
            } else {
                (None, RETAIN_DESIRED_STATE)
            }
        }
        _ => (Some(Ready), REMOVE_DESIRED_STATE),
    }
}
