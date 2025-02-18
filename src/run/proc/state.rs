use std::time;

#[allow(unused)]
#[derive(Clone, Debug, PartialEq)]
pub enum ProcessState {
    Idle,
    HealthCheck(time::Instant),
    Running,
    Failed(Box<ProcessState>),
    /// Retrying at `retry_at`.
    WaitingForRetry(time::Instant),
    Completed,
}
