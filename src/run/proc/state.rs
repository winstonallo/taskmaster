use std::time;

#[allow(unused)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProcessState {
    Idle,
    HealthCheck,
    Running,
    Failed,
    /// Retrying at `retry_at`.
    WaitingForRetry(time::Instant),
    Completed,
}
