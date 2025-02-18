use std::time;

#[derive(Debug)]
pub enum ProcessState {
    Idle,
    Booting,
    Running,
    Failed,
    /// Retrying at `retry_at`.
    WaitingForRetry(time::Instant),
    Completed,
}
