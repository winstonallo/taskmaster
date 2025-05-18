#[allow(unused)]
use super::*;

#[cfg(test)]
mod tests {
    use tokio::{fs::File, io::AsyncReadExt};

    use crate::{conf::Config, run::daemon::Daemon};

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

    #[tokio::test]
    async fn configured_output() {
        let mut proc = ProcessConfig::default();
        let proc = proc
            .set_cmd("python3")
            .set_args(vec!["-c".into(), "import sys;print(f'stdout', flush=True);print(f'stderr',flush=True,file=sys.stderr)".into()])
            .set_stdout("/tmp/stdout.stdout")
            .set_stderr("/tmp/stderr.stderr")
            .set_autostart(true);
        let mut conf = Config::random();
        let conf = conf.add_process("foo", proc.clone());
        let mut daemon = Daemon::from_config(conf.clone(), "bar".into());

        let _ = daemon.run_once().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        let _ = daemon.run_once().await;

        let mut stdout = String::new();
        File::open("/tmp/stdout.stdout")
            .await
            .unwrap()
            .read_to_string(&mut stdout)
            .await
            .unwrap();

        let mut stderr = String::new();
        File::open("/tmp/stderr.stderr")
            .await
            .unwrap()
            .read_to_string(&mut stderr)
            .await
            .unwrap();

        assert_eq!(stdout, "stdout\n".to_string());
        assert_eq!(stderr, "stderr\n".to_string());
    }
}
