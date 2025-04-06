use std::{
    process::Stdio,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use serde::Deserialize;
use tokio::process::Command;

use crate::conf::proc::defaults;

#[derive(Debug, Clone)]
struct CheckStatus {
    started_at: Instant,
    completed: bool,
    exit_code: Option<i32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HealthCheck {
    #[serde(default = "defaults::dflt_startretries")]
    retries: u8,
    #[serde(default = "defaults::dflt_backoff")]
    backoff: u8,
    #[serde(default = "defaults::dflt_timeout")]
    timeout: u8,
    #[serde(default = "defaults::dflt_starttime")]
    starttime: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    args: Option<Vec<String>>,
    #[serde(skip)]
    status: Arc<Mutex<Option<CheckStatus>>>,
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self {
            retries: defaults::dflt_startretries(),
            backoff: defaults::dflt_backoff(),
            timeout: defaults::dflt_timeout(),
            starttime: defaults::dflt_starttime(),
            command: None,
            args: None,
            status: Arc::new(Mutex::new(None)),
        }
    }
}

impl HealthCheck {
    pub fn retries(&self) -> u8 {
        self.retries
    }

    pub fn backoff(&self) -> u8 {
        self.backoff
    }

    pub fn timeout(&self) -> u8 {
        self.timeout
    }

    pub fn starttime(&self) -> u16 {
        self.starttime
    }

    pub fn command(&self) -> Option<String> {
        self.command.clone()
    }

    pub fn args(&self) -> Option<Vec<String>> {
        self.args.clone()
    }

    pub fn running(&self) -> bool {
        if let Ok(status) = self.status.lock() {
            if let Some(check) = &*status {
                return !check.completed;
            }
        }

        false
    }

    pub fn start_background(&self, pid: u32) -> Result<(), String> {
        let cmd = match &self.command {
            Some(cmd) => cmd,
            None => return Ok(()),
        };

        if self.running() {
            return Ok(());
        }

        let status = CheckStatus {
            started_at: Instant::now(),
            completed: false,
            exit_code: None,
        };

        {
            let mut status_guard = self.status.lock().map_err(|e| e.to_string())?;
            *status_guard = Some(status);
        }

        let check_status = Arc::clone(&self.status);
        let cmd = cmd.clone();
        let args = self.args.clone().unwrap_or_default();
        let timeout_duration = Duration::from_secs(self.timeout as u64);
        tokio::spawn(async move {
            let result = tokio::time::timeout(
                timeout_duration,
                Command::new(&cmd)
                    .args(&args)
                    .env("PROCESS_PID", pid.to_string())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status(),
            )
            .await;

            let exit_code = match result {
                Ok(Ok(status)) => status.code(),
                Ok(Err(_)) => None,
                _ => None,
            };

            if let Ok(mut status_guard) = check_status.lock() {
                if let Some(status) = &mut *status_guard {
                    status.completed = true;
                    status.exit_code = exit_code;
                }
            }
        });

        Ok(())
    }

    pub fn check_result(&self) -> Option<(bool, Duration)> {
        let status_guard = match self.status.lock() {
            Ok(guard) => guard,
            Err(_) => return None,
        };

        if let Some(status) = &*status_guard {
            if status.completed {
                let healthy = status.exit_code == Some(0);
                let duration = status.started_at.elapsed();
                return Some((healthy, duration));
            }

            if status.started_at.elapsed() > Duration::from_secs(self.timeout as u64) {
                return Some((false, status.started_at.elapsed()));
            }
        }
        None
    }

    pub fn reset(&self) -> Result<(), String> {
        let mut status_guard = self.status.lock().map_err(|e| e.to_string())?;
        *status_guard = None;
        Ok(())
    }
}
