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
        cmd: String,
        #[serde(default)]
        args: Vec<String>,
    },
    Uptime {
        starttime: u8,
    },
}

#[allow(unused)]
#[derive(Deserialize, Debug, Clone)]
pub struct HealthCheck {
    #[serde(flatten)]
    check: HealthCheckType,
    #[serde(default = "dflt_timeout")]
    timeout: usize,
    #[serde(default = "dflt_retries")]
    retries: usize,
    #[serde(default = "dflt_backoff")]
    backoff: usize,
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self {
            check: HealthCheckType::Uptime { starttime: 5 },
            timeout: dflt_timeout(),
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
            HealthCheckType::Command { cmd: _, args } => args,
            _ => panic!("args() called on an Uptime HealthCheck"),
        }
    }

    pub fn starttime(&self) -> u8 {
        match &self.check {
            HealthCheckType::Uptime { starttime } => *starttime,
            _ => panic!("startime() called on a Command HealthCheck"),
        }
    }

    pub fn timeout(&self) -> usize {
        self.timeout
    }

    pub fn retries(&self) -> usize {
        self.retries
    }

    pub fn backoff(&self) -> usize {
        self.backoff
    }
}
