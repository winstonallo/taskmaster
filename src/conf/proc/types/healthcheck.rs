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
        /// ```toml
        /// cmd = "/usr/bin/ping"
        /// ```
        ///
        /// Required.
        cmd: String,

        /// Arguments to pass to `cmd`.
        ///
        /// ```toml
        /// cmd = "/usr/bin/ping"
        /// args = ["-v"]
        /// ```
        ///
        /// Defaults to `[]`.
        #[serde(default)]
        args: Vec<String>,

        /// Time (in seconds) to let the healthcheck command run through before
        /// considering it failed.
        ///
        /// Arguments to pass to `cmd`.
        ///
        /// ```toml
        /// cmd = "/usr/bin/ping"
        /// timeout = 10
        /// ```
        ///
        /// Defaults to `[]`.
        ///
        /// Defaults to `10`.
        #[serde(default = "dflt_timeout")]
        timeout: usize,
    },
    Uptime {
        /// Time (in seconds) after which the process will be deemed healthy.
        ///
        /// starttime = 10
        ///
        /// Defaults to `1`, max `65536`.
        starttime: u16,
    },
}

#[allow(unused)]
#[derive(Deserialize, Debug, Clone)]
pub struct HealthCheck {
    /// Inferred from the configured fields.
    ///
    /// Must be one of:
    /// - Command: Pass/fail based on `cmd`'s exit status.
    /// ```toml
    /// cmd = <string>
    /// args = <<string>>
    /// timeout = <int>
    /// ```
    /// - Uptime: Consider healthy after running for `startttime` seconds.
    ///
    /// ```toml
    /// starttime = <int>
    /// ```
    #[serde(flatten)]
    check: HealthCheckType,

    /// How many times to retry the healthcheck before giving up.
    ///
    /// ```toml
    /// [processes.nginx.healthcheck]
    /// cmd = "/usr/bin/ping"
    /// args = ["localhost"]
    /// retries = 3
    /// ```
    ///
    /// Defaults to `5`.
    #[serde(default = "dflt_retries")]
    retries: usize,

    /// Time (in seconds) to wait after a failed healthcheck before retrying.
    ///
    /// ```toml
    /// [processes.nginx.healthcheck]
    /// starttime = 3
    /// backoff = 3
    /// ```
    ///
    /// Defaults to `5`.
    #[serde(default = "dflt_backoff")]
    backoff: usize,
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self {
            check: HealthCheckType::Uptime { starttime: 1 },
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

#[cfg(test)]
impl HealthCheck {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_check(&mut self, check: HealthCheckType) -> &mut Self {
        self.check = check;
        self
    }

    pub fn set_backoff(&mut self, backoff: usize) -> &mut Self {
        self.backoff = backoff;
        self
    }

    pub fn set_retries(&mut self, retries: usize) -> &mut Self {
        self.retries = retries;
        self
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    impl HealthCheck {
        pub fn command() -> Self {
            Self {
                check: HealthCheckType::Command {
                    cmd: "/usr/bin/echo".to_string(),
                    args: Vec::new(),
                    timeout: 5,
                },
                retries: 5,
                backoff: 5,
            }
        }

        pub fn uptime() -> Self {
            Self {
                check: HealthCheckType::Uptime { starttime: 1 },
                retries: 5,
                backoff: 5,
            }
        }
    }

    #[test]
    fn cmd_on_command_healthcheck() {
        let hc = HealthCheck::command();
        assert_eq!(hc.cmd(), "/usr/bin/echo".to_string());
    }

    #[test]
    fn retries() {
        let hc = HealthCheck::command();
        assert_eq!(hc.retries(), 5);
    }

    #[test]
    fn backoff() {
        let hc = HealthCheck::command();
        assert_eq!(hc.backoff(), 5);
    }

    #[test]
    fn args_on_command_healthcheck() {
        let hc = HealthCheck::command();
        assert_eq!(hc.args(), Vec::<String>::new());
    }

    #[test]
    fn timeout_on_command_healthcheck() {
        let hc = HealthCheck::command();
        assert_eq!(hc.timeout(), 5);
    }

    #[test]
    fn starttime_on_uptime_healthcheck() {
        let hc = HealthCheck::uptime();
        assert_eq!(hc.starttime(), 1);
    }

    #[test]
    #[should_panic]
    fn cmd_on_uptime_healthcheck() {
        let healthcheck = HealthCheck::uptime();
        healthcheck.cmd();
    }

    #[test]
    #[should_panic]
    fn args_on_uptime_healthcheck() {
        let healthcheck = HealthCheck::uptime();
        healthcheck.args();
    }

    #[test]
    #[should_panic]
    fn timeout_on_uptime_healthcheck() {
        let healthcheck = HealthCheck::uptime();
        healthcheck.timeout();
    }

    #[test]
    #[should_panic]
    fn starttime_on_command_healthcheck() {
        let healthcheck = HealthCheck::command();
        healthcheck.starttime();
    }
}
