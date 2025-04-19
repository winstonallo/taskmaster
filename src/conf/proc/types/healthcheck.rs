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

#[allow(unused)]
#[derive(Deserialize, Debug, Clone)]
pub struct HealthCheck {
    cmd: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default = "dflt_timeout")]
    timeout: usize,
    #[serde(default = "dflt_retries")]
    retries: usize,
    #[serde(default = "dflt_backoff")]
    backoff: usize,
}

impl HealthCheck {
    pub fn cmd(&self) -> &str {
        &self.cmd
    }

    pub fn args(&self) -> &[String] {
        &self.args
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
