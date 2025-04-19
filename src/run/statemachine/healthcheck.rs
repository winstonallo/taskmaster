#![allow(unused)]
use std::time::Instant;

#[derive(Debug)]
pub struct CommandResult {
    exitcode: i32,
    msg: String,
}

#[derive(Debug)]
pub struct HealthCheckEvent {
    process_name: String,
    event_type: HealthCheckEventType,
    timestamp: Instant,
    result: Option<CommandResult>, // only present for Passed/Failed events
}

#[derive(Debug)]
pub enum HealthCheckEventType {
    Passed,
    Failed(String),
    TimeOut,
}
