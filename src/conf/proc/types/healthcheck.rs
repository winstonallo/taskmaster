use serde::Deserialize;

fn dflt_timeout() -> usize {
    10
}

fn dflt_backoff() -> usize {
    5
}

fn dflt_retries() -> usize {
    5
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum HealthCheckType {
    Command {
        /// Command to run as a healthcheck.
        ///
        /// Required.
        cmd: String,

        /// Arguments to pass to `cmd`.
        ///
        /// Defaults to `[]`.
        #[serde(default)]
        args: Vec<String>,

        /// Time, in seconds, to let the healthcheck command run through before considering
        /// it failed.
        ///
        /// Defaults to 10 seconds.
        #[serde(default = "dflt_timeout")]
        timeout: usize,
    },
    Uptime {
        /// Time (in seconds) after which the process will be deemed healthy.
        ///
        /// Defaults to `5`, max `65536`.
        starttime: u16,
    },
}

#[allow(unused)]
#[derive(Deserialize, Debug, Clone)]
pub struct HealthCheck {
    #[serde(flatten)]
    check: HealthCheckType,
    #[serde(default = "dflt_retries")]
    retries: usize,
    #[serde(default = "dflt_backoff")]
    backoff: usize,
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self {
            check: HealthCheckType::Uptime { starttime: 5 },
            retries: dflt_retries(),
            backoff: dflt_backoff(),
        }
    }
}

impl HealthCheck {
    pub fn healthcheck(&self) -> &HealthCheckType {
        &self.check
    }

    pub fn cmd(&self) -> &str {
        match &self.check {
            HealthCheckType::Command { cmd, .. } => cmd,
            _ => panic!("cmd() called on an Uptime HealthCheck"),
        }
    }

    pub fn args(&self) -> &[String] {
        match &self.check {
            HealthCheckType::Command { cmd: _, args, timeout: _ } => args,
            _ => panic!("args() called on an Uptime HealthCheck"),
        }
    }

    pub fn starttime(&self) -> u16 {
        match &self.check {
            HealthCheckType::Uptime { starttime } => *starttime,
            _ => panic!("starttime() called on a Command HealthCheck"),
        }
    }

    pub fn timeout(&self) -> usize {
        match &self.check {
            HealthCheckType::Command { cmd: _, args: _, timeout } => *timeout,
            _ => panic!("timeout() called on an Uptime HealthCheck"),
        }
    }

    pub fn retries(&self) -> usize {
        self.retries
    }

    pub fn backoff(&self) -> usize {
        self.backoff
    }
}
