#![allow(unused)]
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct HealthCheckRunner {
    failures: usize,
    task: Option<tokio::task::JoinHandle<()>>,
    receiver: Option<tokio::sync::oneshot::Receiver<HealthCheckEvent>>,
    timeout: usize,
    retries: usize,
}

#[derive(Debug)]
pub enum HealthCheckEvent {
    Passed,
    Failed(String),
    TimeOut,
}

impl HealthCheckRunner {
    pub fn new(timeout: usize, retries: usize) -> Self {
        Self {
            failures: 0,
            task: None,
            receiver: None,
            timeout,
            retries,
        }
    }

    pub fn failures(&self) -> usize {
        self.failures
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
            Ok(Ok(output)) => {
                HealthCheckEvent::Failed(format!("healthcheck failed with status: {}, stderr: {}", output.status, String::from_utf8_lossy(&output.stderr)))
            }
            Ok(Err(e)) => HealthCheckEvent::Failed(format!("healthcheck failed to execute: {e}")),
            Err(_) => HealthCheckEvent::TimeOut,
        }
    }

    pub fn start(&mut self, cmd: &str, args: &[String], timeout: usize) {
        let (sender, receiver) = tokio::sync::oneshot::channel::<HealthCheckEvent>();
        self.receiver = Some(receiver);

        let cmd = cmd.to_string();
        let args = args.to_vec();
        let timeout = Duration::from_secs(timeout as u64);

        let handle = tokio::task::spawn(async move {
            let result = HealthCheckRunner::spawn(&cmd, &args, timeout).await;
            let _ = sender.send(result);
        });

        self.task = Some(handle);
    }
}
