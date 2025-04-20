#![allow(unused)]
use std::{
    time::{Duration, Instant},
    vec,
};

use crate::conf::proc::types::HealthCheck;

#[derive(Debug)]
pub struct HealthCheckRunner {
    failures: usize,
    task: Option<tokio::task::JoinHandle<()>>,
    receiver: Option<tokio::sync::oneshot::Receiver<HealthCheckEvent>>,
    cmd: String,
    args: Vec<String>,
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
            cmd: hc.cmd().to_string(),
            args: hc.args().to_vec(),
            timeout: hc.timeout(),
            retries: hc.retries(),
            backoff: hc.backoff(),
        }
    }

    pub fn cmd(&self) -> &str {
        &self.cmd
    }

    pub fn args(&self) -> &Vec<String> {
        &self.args
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

        let cmd = self.cmd.clone();
        let args = self.args.clone();
        let timeout = Duration::from_secs(self.timeout as u64);

        let handle = tokio::task::spawn(async move {
            let result = HealthCheckRunner::spawn(&cmd, &args, timeout).await;
            let _ = sender.send(result);
        });

        self.task = Some(handle);
    }
}
