#![allow(unused)]
use std::time::Instant;

struct CommandResult {
    exitcode: i32,
    msg: String,
}

struct HealthCheckEvent {
    process_name: String,
    event_type: HealthCheckEventType,
    timestamp: Instant,
    result: Option<CommandResult>, // only present for Passed/Failed events
}

enum HealthCheckEventType {
    Requested,
    Started,
    Passed,
    Failed,
    TimedOut,
}
