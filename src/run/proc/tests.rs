#[allow(unused)]
use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_command_healthcheck() {
        let proc = Process {
            id: None,
            name: "name".to_string(),
            child: None,
            conf: ProcessConfig::testconfig(),
            healthcheck: HealthCheckRunner::uptime(),
            runtime_failures: 0,
            state: ProcessState::Idle,
            desired_states: VecDeque::new(),
        };

        assert!(!proc.has_command_healthcheck());
    }
}
