use std::{collections::HashMap, error::Error, sync::Arc, time::Duration};

use serde::{Deserialize, Serialize};
use socket::AsyncUnixSocket;
use tokio::time::sleep;

use super::proc::{self, Process};
use super::statemachine::states::ProcessState;
use crate::jsonrpc::response::{Response, ResponseError, ResponseType};
use crate::{
    conf,
    jsonrpc::{handlers::handle_request, request::Request},
    log_error,
};
mod error;
mod socket;

pub struct Daemon {
    processes: HashMap<String, proc::Process>,
    socket_path: String,
    auth_group: String,
    config_path: String,
    shutting_down: bool,
}

impl Daemon {
    pub fn from_config(conf: conf::Config, config_path: String) -> Self {
        let processes: HashMap<String, proc::Process> = conf
            .processes()
            .iter()
            .flat_map(|(proc_name, proc)| {
                (0..proc.processes()).map(move |id| {
                    let key = if proc.processes() > 1 {
                        format!("{}_{}", proc_name, id)
                    } else {
                        proc_name.to_owned()
                    };
                    (key.clone(), proc::Process::from_process_config(proc.clone(), &key))
                })
            })
            .collect::<HashMap<String, proc::Process>>();

        Self {
            processes,
            socket_path: conf.socketpath().to_owned(),
            auth_group: conf.authgroup().to_owned(),
            config_path,
            shutting_down: false,
        }
    }

    pub fn processes(&self) -> &HashMap<String, Process> {
        &self.processes
    }

    pub fn processes_mut(&mut self) -> &mut HashMap<String, Process> {
        &mut self.processes
    }

    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }

    pub fn auth_group(&self) -> &str {
        &self.auth_group
    }

    pub fn config_path(&self) -> &str {
        &self.config_path
    }

    pub fn shutting_down(&self) -> bool {
        self.shutting_down
    }

    pub fn shutdown(&mut self) {
        self.shutting_down = true;
    }

    #[cfg(test)]
    pub async fn run_once(&mut self) -> Result<(), Box<dyn Error + Send>> {
        let mut listener = AsyncUnixSocket::new(self.socket_path(), self.auth_group()).unwrap();

        let (sender, mut receiver) = tokio::sync::mpsc::channel(1024);
        let sender = Arc::new(sender);

        tokio::select! {
            accept_result = listener.accept() => {

                if let Err(e) = accept_result {
                    log_error!("Failed to accept connection: {}", e);
                }

                let mut socket = listener;
                let clone = sender.clone();

                let shutting_down = self.shutting_down;
                tokio::spawn(async move {
                    if shutting_down {

                        let _ = socket.write("not accepting requests - currently shutting down".as_bytes()).await;
                    } else {
                        handle_client(socket, clone).await;
                    }
                });
            },

            Some((request, mut socket)) = receiver.recv() => {
                let response = handle_request(self, request);

                let msg = serde_json::to_string(&response).unwrap();

                tokio::spawn(async move {
                    if let Err(e) = socket.write(msg.as_bytes()).await {
                        log_error!("error sending to socket: {}", e);
                    }
                });
            },

            _ = sleep(Duration::from_nanos(1)) => {
                monitor_state(self.processes_mut());

                if  self.shutting_down && self.no_process_running(){
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let mut listener = AsyncUnixSocket::new(self.socket_path(), self.auth_group()).unwrap();

        let (sender, mut receiver) = tokio::sync::mpsc::channel(1024);
        let sender = Arc::new(sender);

        loop {
            tokio::select! {
                accept_result = listener.accept() => {

                    if let Err(e) = accept_result {
                        log_error!("Failed to accept connection: {}", e);
                        continue;
                    }

                    let mut socket = listener;
                    let clone = sender.clone();

                    let shutting_down = self.shutting_down;
                    tokio::spawn(async move {
                        if shutting_down {

                            let _ = socket.write("not accepting requests - currently shutting down".as_bytes()).await;
                        } else {
                            handle_client(socket, clone).await;
                        }
                    });

                    listener = AsyncUnixSocket::new(self.socket_path(), self.auth_group())?;
                },

                Some((request, mut socket)) = receiver.recv() => {
                    let response = handle_request(self, request);

                    let msg = serde_json::to_string(&response).unwrap();

                    tokio::spawn(async move {
                        if let Err(e) = socket.write(msg.as_bytes()).await {
                            log_error!("error sending to socket: {}", e);
                        }
                    });
                },

                _ = sleep(Duration::from_nanos(1)) => {
                    monitor_state(self.processes_mut());

                    if  self.shutting_down && self.no_process_running(){
                        return Ok(());
                    }
                }
            }
        }
    }

    pub fn no_process_running(&self) -> bool {
        let mut no_process_running = true;
        for proc in self.processes().values() {
            use ProcessState::*;
            match proc.state() {
                Ready | HealthCheck(_) | Healthy | Stopping(_) => no_process_running = false,
                _ => {}
            }
        }
        no_process_running
    }
}

fn monitor_state(procs: &mut HashMap<String, Process>) {
    for proc in procs.values_mut() {
        proc.desire();
        proc.monitor();
    }
}

#[derive(Serialize, Deserialize)]
pub struct MinimumRequest {
    pub id: u32,
}

async fn handle_client(mut socket: AsyncUnixSocket, sender: Arc<tokio::sync::mpsc::Sender<(Request, AsyncUnixSocket)>>) {
    let mut line = String::new();

    match socket.read_line(&mut line).await {
        Ok(0) => { /* connection closed, do nothing */ }
        Ok(_) => match serde_json::from_str(&line) {
            Ok(request) => {
                let _ = sender.send((request, socket)).await;
            }
            Err(e) => {
                let error_msg = match serde_json::from_str::<MinimumRequest>(&line) {
                    Ok(m_r) => serde_json::to_string(&Response::new(
                        m_r.id,
                        ResponseType::Error(ResponseError {
                            code: crate::jsonrpc::response::ErrorCode::InvalidRequest,
                            message: format!("{}", e).to_owned(),
                            data: None,
                        }),
                    ))
                    .unwrap(),
                    Err(_) => "request id not found - can't respond with JsonRPCError".to_owned(),
                };
                if let Err(e) = socket.write(error_msg.as_bytes()).await {
                    log_error!("error writing to socket: {}", e)
                }
            }
        },
        Err(e) => {
            log_error!("Error reading from socket: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::{Rng, distr::Alphanumeric};

    use crate::conf::proc::ProcessConfig;
    use crate::conf::proc::types::{HealthCheck, HealthCheckType};

    use super::conf::Config;

    use super::*;

    impl Config {
        fn random() -> Config {
            let socketpath = rand::rng().sample_iter(&Alphanumeric).take(8).map(char::from).collect::<String>();

            Self::default().set_socketpath(&format!("/tmp/{socketpath}.sock")).to_owned()
        }
    }

    #[tokio::test]
    async fn idle_to_healthcheck() {
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sleep").set_args(vec!["2".to_string()]);
        let mut conf = Config::random();
        let conf = conf.add_process("sleep", proc.clone());
        let mut d = Daemon::from_config(conf.clone(), "idc".to_string());

        let _ = d.run_once().await;

        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::HealthCheck(_)));
        d.shutdown();
    }

    #[tokio::test]
    async fn healthcheck_to_healthy_uptime() {
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sleep").set_args(vec!["2".to_string()]);
        let mut conf = Config::random();
        let conf = conf.add_process("sleep", proc.clone());
        let mut d = Daemon::from_config(conf.clone(), "idc".to_string());

        let _ = d.run_once().await;
        tokio::time::sleep(Duration::from_secs(1)).await;
        let _ = d.run_once().await;

        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Healthy);
        d.shutdown();
    }

    #[tokio::test]
    async fn healthcheck_to_completed_uptime() {
        let mut hc = HealthCheck::default();
        let hc = hc.set_check(HealthCheckType::Uptime { starttime: 2 });
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sleep").set_args(vec!["1".to_string()]).set_healthcheck(hc.to_owned());
        let mut conf = Config::random();
        let conf = conf.add_process("sleep", proc.clone());
        let mut d = Daemon::from_config(conf.clone(), "idc".to_string());

        let _ = d.run_once().await;
        tokio::time::sleep(Duration::from_secs(1)).await;
        let _ = d.run_once().await;

        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Completed);
        d.shutdown();
    }

    #[tokio::test]
    async fn healthcheck_to_failed_uptime() {
        let mut hc = HealthCheck::default();
        let hc = hc.set_check(HealthCheckType::Uptime { starttime: 2 });
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sh").set_args(vec!["-c".to_string(), "sleep 1 && exit 1".to_string()]).set_healthcheck(hc.to_owned());
        let mut conf = Config::random();
        let conf = conf.add_process("sleep", proc.clone());
        let mut d = Daemon::from_config(conf.clone(), "idc".to_string());

        let _ = d.run_once().await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::HealthCheck(_)));
        tokio::time::sleep(Duration::from_millis(600)).await;
        let _ = d.run_once().await;

        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::Failed(_)));
        d.shutdown();
    }

    #[tokio::test]
    async fn healthcheck_to_failed_retry() {
        let mut hc = HealthCheck::default();
        let hc = hc.set_check(HealthCheckType::Uptime { starttime: 2 }).set_backoff(1).set_retries(2);
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sh").set_args(vec!["-c".to_string(), "sleep 1 && exit 1".to_string()]).set_healthcheck(hc.to_owned());
        let mut conf = Config::random();
        let conf = conf.add_process("sleep", proc.clone());
        let mut d = Daemon::from_config(conf.clone(), "idc".to_string());

        let _ = d.run_once().await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::HealthCheck(_)));
        tokio::time::sleep(Duration::from_millis(600)).await;
        let _ = d.run_once().await;

        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::Failed(_)));
        let _ = d.run_once().await;

        assert_eq!(d.processes().get("sleep").unwrap().healthcheck_failures(), 1);

        // Wait for backoff, process should then be back in healthcheck.
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::HealthCheck(_)));
        d.shutdown();
    }

    #[tokio::test]
    async fn healthcheck_to_stopped_max_retries_reached() {
        let mut hc = HealthCheck::default();
        let hc = hc.set_check(HealthCheckType::Uptime { starttime: 2 }).set_backoff(1).set_retries(1);
        let mut proc = ProcessConfig::default();
        let proc = proc
            .set_cmd("sh")
            .set_args(vec!["-c".to_string(), "sleep 1 && exit 1".to_string()])
            .set_healthcheck(hc.to_owned())
            .set_stoptime(1);
        let mut conf = Config::random();
        let conf = conf.add_process("sleep", proc.clone());
        let mut d = Daemon::from_config(conf.clone(), "idc".to_string());

        // Start process and healthcheck.
        let _ = d.run_once().await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::HealthCheck(_)));
        tokio::time::sleep(Duration::from_millis(600)).await;

        // Run to catch failure.
        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::Failed(_)));

        // Run to enter monitor_failed and increment failure count and push desired Stopped state.
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().healthcheck_failures(), 1);

        // Run one last time to catch Stopped state (the Stopping -> Stopped transition is done
        // in the same iteration of the state machine).
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Stopped);

        d.shutdown();
    }

    #[tokio::test]
    async fn healthy_to_failed() {
        let mut hc = HealthCheck::default();
        let hc = hc.set_check(HealthCheckType::Uptime { starttime: 1 });
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sh").set_args(vec!["-c".to_string(), "sleep 2; exit 1".to_string()]).set_healthcheck(hc.to_owned());
        let mut conf = Config::random();
        let conf = conf.add_process("sleep", proc.clone());
        let mut d = Daemon::from_config(conf.clone(), "idc".to_string());

        let _ = d.run_once().await;

        // Wait for process to be healthy.
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Healthy);

        // Wait for the process to exit with bad status.
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let _ = d.run_once().await;
        let _ = d.run_once().await;

        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::Failed(_)));
        d.shutdown();
    }

    #[tokio::test]
    async fn healthy_to_completed() {
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sleep").set_args(vec!["2".to_string()]);
        let mut conf = Config::random();
        let conf = conf.add_process("sleep", proc.clone());
        let mut d = Daemon::from_config(conf.clone(), "idc".to_string());

        let _ = d.run_once().await;
        tokio::time::sleep(Duration::from_secs(2)).await;
        let _ = d.run_once().await;

        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Completed);
        d.shutdown();
    }

    #[tokio::test]
    async fn healthy_to_stopped() {
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sleep").set_args(vec!["10".to_string()]).set_stoptime(1);
        let mut conf = Config::random();
        let conf = conf.add_process("sleep", proc.clone());
        let mut d = Daemon::from_config(conf.clone(), "idc".to_string());

        // Run once to start the process and wait for `starttime`.
        let _ = d.run_once().await;
        tokio::time::sleep(Duration::from_millis(1100)).await;

        // Run once to update the state.
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Healthy);

        // Push desired Stopped state, run once to update state.
        let _ = d.processes_mut().get_mut("sleep").unwrap().push_desired_state(ProcessState::Stopped);
        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::Stopping(_)));

        // Wait for `stoptime` and run once to update state.
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Stopped);

        d.shutdown();
    }

    #[tokio::test]
    async fn healthcheck_to_healthy_command() {
        let mut hc = HealthCheck::default();
        let hc = hc.set_check(HealthCheckType::Command {
            cmd: "sleep".to_string(),
            args: vec!["1".to_string()],
            timeout: 10,
        });
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sleep").set_args(vec!["10".to_string()]).set_healthcheck(hc.to_owned());
        let mut conf = Config::random();
        conf.add_process("sleep", proc.to_owned());
        let mut d = Daemon::from_config(conf, "foo".to_string());

        // Run once to get into the HealthCheck state.
        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::HealthCheck(_)));

        // Run once again to trigger the healthcheck command (happens on the first iteration of HealthCheck).
        let _ = d.run_once().await;

        // Sleep and run once again to verify that the process is now healthy.
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Healthy);
    }

    #[tokio::test]
    async fn healthcheck_to_failed_command_timeout() {
        let mut hc = HealthCheck::default();
        let hc = hc.set_check(HealthCheckType::Command {
            cmd: "sleep".to_string(),
            args: vec!["2".to_string()],
            timeout: 1,
        });
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sleep").set_args(vec!["10".to_string()]).set_healthcheck(hc.to_owned());
        let mut conf = Config::random();
        conf.add_process("sleep", proc.to_owned());
        let mut d = Daemon::from_config(conf, "foo".to_string());

        // Run once to get into the HealthCheck state.
        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::HealthCheck(_)));

        // Run once again to trigger the healthcheck command (happens on the first iteration of HealthCheck).
        let _ = d.run_once().await;

        // Sleep and run once again to verify that the process is not healthy.
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::Failed(_)));
    }

    #[tokio::test]
    async fn healthcheck_to_failed_command() {
        let mut hc = HealthCheck::default();
        let hc = hc.set_check(HealthCheckType::Command {
            cmd: "sleep".to_string(),
            args: vec!["asd".to_string()], // Will fail right away.
            timeout: 1,
        });
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sleep").set_args(vec!["10".to_string()]).set_healthcheck(hc.to_owned());
        let mut conf = Config::random();
        conf.add_process("sleep", proc.to_owned());
        let mut d = Daemon::from_config(conf, "foo".to_string());

        // Run once to get into the HealthCheck state.
        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::HealthCheck(_)));

        // Run once again to trigger the healthcheck command (happens on the first iteration of HealthCheck).
        let _ = d.run_once().await;

        // Sleep and run once again to verify that the healthcheck failed.
        tokio::time::sleep(Duration::from_millis(10)).await;
        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::Failed(_)));
    }

    #[tokio::test]
    async fn healthcheck_to_completed() {
        let mut hc = HealthCheck::default();
        let hc = hc.set_check(HealthCheckType::Command {
            cmd: "sleep".to_string(),
            args: vec!["10".to_string()],
            timeout: 10,
        });
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sleep").set_args(vec!["1".to_string()]).set_healthcheck(hc.to_owned());
        let mut conf = Config::random();
        conf.add_process("sleep", proc.to_owned());
        let mut d = Daemon::from_config(conf, "foo".to_string());

        // Run once to get into the HealthCheck state.
        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::HealthCheck(_)));

        // Run once again to trigger the healthcheck command (happens on the first iteration of HealthCheck).
        let _ = d.run_once().await;

        // Sleep and run once again to verify that the healthcheck failed.
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Completed);
    }

    #[tokio::test]
    async fn health_to_completed() {
        let mut hc = HealthCheck::default();
        let hc = hc.set_check(HealthCheckType::Command {
            cmd: "sleep".to_string(),
            args: vec!["10".to_string()],
            timeout: 10,
        });
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sleep").set_args(vec!["1".to_string()]).set_healthcheck(hc.to_owned());
        let mut conf = Config::random();
        conf.add_process("sleep", proc.to_owned());
        let mut d = Daemon::from_config(conf, "foo".to_string());

        // Run once to get into the HealthCheck state.
        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::HealthCheck(_)));

        // Run once again to trigger the healthcheck command (happens on the first iteration of HealthCheck).
        let _ = d.run_once().await;

        // Sleep and run once again to verify that the healthcheck failed.
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Completed);
    }
}
