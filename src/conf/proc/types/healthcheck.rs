use serde::Deserialize;

use crate::conf::proc::defaults;

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
    #[serde(skip_deserializing)]
    command: Option<String>,
    #[serde(skip_deserializing)]
    args: Option<Vec<String>>,
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
}
