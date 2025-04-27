#![allow(unused)]
use serde::Deserialize;
use std::{
    time::{Duration, Instant},
    vec,
};

use crate::conf::proc::types::{HealthCheck, HealthCheckType};

#[derive(Debug)]
pub struct HealthCheckRunner {
    failures: usize,
    task: Option<tokio::task::JoinHandle<()>>,
    receiver: Option<tokio::sync::oneshot::Receiver<HealthCheckEvent>>,
    check: HealthCheckType,
    retries: usize,
    backoff: usize,
}

#[derive(Debug)]
pub enum HealthCheckEvent {
    Passed,
    Failed(String),
}

impl HealthCheckRunner {
    pub fn from_healthcheck_config(hc: &HealthCheck) -> Self {
        Self {
            failures: 0,
            task: None,
            receiver: None,
            check: hc.healthcheck().clone(),
            retries: hc.retries(),
            backoff: hc.backoff(),
        }
    }

    pub fn has_command_healthcheck(&self) -> bool {
        matches!(self.check, HealthCheckType::Command { .. })
    }

    pub fn cmd(&self) -> String {
        match &self.check {
            HealthCheckType::Command { cmd, .. } => cmd.to_string(),
            _ => panic!("cmd() called on an Uptime HealthCheck"),
        }
    }

    pub fn args(&self) -> Vec<String> {
        match &self.check {
            HealthCheckType::Command { cmd: _, args, .. } => args.clone(),
            _ => panic!("args() called on an Uptime HealthCheck"),
        }
    }

    pub fn timeout(&self) -> usize {
        match &self.check {
            HealthCheckType::Command { cmd: _, args: _, timeout } => *timeout,
            _ => panic!("timeout() called on an Uptime HealthCheck"),
        }
    }

    pub fn starttime(&self) -> u16 {
        match &self.check {
            HealthCheckType::Uptime { starttime } => *starttime,
            _ => panic!("starttime() called on a Command HealthCheck"),
        }
    }

    pub fn failures(&self) -> usize {
        self.failures
    }

    pub fn retries(&self) -> usize {
        self.retries
    }

    pub fn backoff(&self) -> usize {
        self.backoff
    }

    pub fn increment_failures(&mut self) {
        self.failures = self.failures.saturating_add(1);
    }

    pub fn task(&self) -> &Option<tokio::task::JoinHandle<()>> {
        &self.task
    }

    pub fn receiver(&mut self) -> &mut Option<tokio::sync::oneshot::Receiver<HealthCheckEvent>> {
        &mut self.receiver
    }

    pub fn clear(&mut self) {
        self.task = None;
        self.receiver = None;
    }

    async fn spawn(cmd: &str, args: &Vec<String>, timeout: Duration) -> HealthCheckEvent {
        let mut command = tokio::process::Command::new(cmd);
        command.args(args);

        match tokio::time::timeout(timeout, command.output()).await {
            Ok(Ok(output)) if output.status.success() => HealthCheckEvent::Passed,
            Ok(Ok(output)) => HealthCheckEvent::Failed(format!("exit code: {}, stderr: {}", output.status, String::from_utf8_lossy(&output.stderr))),
            Ok(Err(e)) => HealthCheckEvent::Failed(e.to_string()),
            Err(_) => HealthCheckEvent::Failed(format!("timed out after {} seconds", timeout.as_secs())),
        }
    }

    pub fn start(&mut self) {
        assert!(matches!(self.check, HealthCheckType::Command { .. }));

        let (sender, receiver) = tokio::sync::oneshot::channel::<HealthCheckEvent>();
        self.receiver = Some(receiver);

        let cmd = self.cmd().clone();
        let args = self.args().to_vec();
        let timeout = Duration::from_secs(self.timeout() as u64);

        let handle = tokio::task::spawn(async move {
            let result = HealthCheckRunner::spawn(&cmd, &args, timeout).await;
            let _ = sender.send(result);
        });

        self.task = Some(handle);
    }
}

#[cfg(test)]
mod tests {
    use tokio::time::sleep;

    use super::*;

    impl HealthCheckRunner {
        pub fn command() -> Self {
            Self {
                check: HealthCheckType::Command {
                    cmd: "/usr/bin/echo".to_string(),
                    args: Vec::new(),
                    timeout: 5,
                },
                failures: 0,
                task: None,
                receiver: None,
                retries: 5,
                backoff: 5,
            }
        }

        pub fn uptime() -> Self {
            Self {
                check: HealthCheckType::Uptime { starttime: 5 },
                failures: 0,
                task: None,
                receiver: None,
                retries: 5,
                backoff: 5,
            }
        }
    }

    #[test]
    fn increment_failures() {
        let mut hc = HealthCheckRunner::command();
        hc.increment_failures();
        assert_eq!(hc.failures(), 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn clear() {
        let mut hc = HealthCheckRunner::command();
        hc.start();
        hc.clear();
        assert!(hc.task().is_none());
        assert!(hc.receiver().is_none());
    }

    #[test]
    fn cmd_on_command_healthcheck() {
        let hc = HealthCheckRunner::command();
        assert_eq!(hc.cmd(), "/usr/bin/echo".to_string());
    }

    #[test]
    fn retries() {
        let hc = HealthCheckRunner::command();
        assert_eq!(hc.retries(), 5);
    }

    #[test]
    fn backoff() {
        let hc = HealthCheckRunner::command();
        assert_eq!(hc.backoff(), 5);
    }

    #[test]
    fn args_on_command_healthcheck() {
        let hc = HealthCheckRunner::command();
        assert_eq!(hc.args(), Vec::<String>::new());
    }

    #[test]
    fn timeout_on_command_healthcheck() {
        let hc = HealthCheckRunner::command();
        assert_eq!(hc.timeout(), 5);
    }

    #[test]
    fn starttime_on_uptime_healthcheck() {
        let hc = HealthCheckRunner::uptime();
        assert_eq!(hc.starttime(), 5);
    }

    #[test]
    #[should_panic]
    fn start_on_uptime_healthcheck() {
        let mut healthcheck = HealthCheckRunner::uptime();
        healthcheck.start();
    }

    #[test]
    #[should_panic]
    fn cmd_on_uptime_healthcheck() {
        let mut healthcheck = HealthCheckRunner::uptime();
        healthcheck.cmd();
    }

    #[test]
    #[should_panic]
    fn args_on_uptime_healthcheck() {
        let mut healthcheck = HealthCheckRunner::uptime();
        healthcheck.args();
    }

    #[test]
    #[should_panic]
    fn timeout_on_uptime_healthcheck() {
        let mut healthcheck = HealthCheckRunner::uptime();
        healthcheck.timeout();
    }

    #[test]
    #[should_panic]
    fn starttime_on_command_healthcheck() {
        let mut healthcheck = HealthCheckRunner::command();
        healthcheck.starttime();
    }
}
