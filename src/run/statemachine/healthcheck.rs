#![allow(unused)]
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
    timeout: usize,
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
            timeout: hc.timeout(),
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
            HealthCheckType::Command { cmd: _, args } => args.clone(),
            _ => panic!("args() called on an Uptime HealthCheck"),
        }
    }

    pub fn starttime(&self) -> u8 {
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
        let (sender, receiver) = tokio::sync::oneshot::channel::<HealthCheckEvent>();
        self.receiver = Some(receiver);

        let cmd = self.cmd().clone();
        let args = self.args().to_vec();
        let timeout = Duration::from_secs(self.timeout as u64);

        let handle = tokio::task::spawn(async move {
            let result = HealthCheckRunner::spawn(&cmd, &args, timeout).await;
            let _ = sender.send(result);
        });

        self.task = Some(handle);
    }
}
