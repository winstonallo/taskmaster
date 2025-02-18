use std::time;

#[derive(Debug)]
pub enum ProcessState {
    Idle,
    HealthCheck,
    Running,
    Failed,
    /// Retrying at `retry_at`.
    WaitingForRetry(time::Instant),
    Completed,
}
