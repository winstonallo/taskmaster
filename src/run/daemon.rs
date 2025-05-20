use std::sync::atomic::{AtomicBool, Ordering};
use std::{collections::HashMap, error::Error, sync::Arc, time::Duration};

use libc::SIGHUP;
use serde::{Deserialize, Serialize};
use socket::AsyncUnixSocket;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::signal::unix::{SignalKind, signal};
use tokio::time::sleep;

use super::proc::{self, Process};
use super::statemachine::states::ProcessState;
use crate::conf::proc::types::AuthGroup;
use crate::conf::{Config, PID_FILE_PATH};
use crate::jsonrpc::handlers::AttachmentManager;
use crate::jsonrpc::response::{Response, ResponseError, ResponseType};
use crate::log_info;
use crate::{
    conf,
    jsonrpc::{handlers::handle_request, request::Request},
    log_error,
};
mod error;
pub mod socket;

pub struct Daemon {
    processes: HashMap<String, proc::Process>,
    socket_path: String,
    auth_group: Option<AuthGroup>,
    config_path: String,
    shutting_down: bool,
    attachment_manager: AttachmentManager,
}

impl Daemon {
    pub fn from_config(conf: conf::Config, config_path: String) -> Self {
        let processes: HashMap<String, proc::Process> = conf
            .processes()
            .iter()
            .flat_map(|(proc_name, proc)| {
                (0..proc.processes()).map(move |id| {
                    let key = if proc.processes() > 1 {
                        format!("{proc_name}_{id}")
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
            attachment_manager: AttachmentManager::new(),
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

    pub fn auth_group(&self) -> &Option<AuthGroup> {
        &self.auth_group
    }

    pub fn config_path(&self) -> &str {
        &self.config_path
    }

    pub fn shutting_down(&self) -> bool {
        self.shutting_down
    }

    pub fn shutdown(&mut self) {
        let _ = std::fs::remove_file(PID_FILE_PATH);
        self.shutting_down = true;
    }

    pub fn attachment_manager(&self) -> &AttachmentManager {
        &self.attachment_manager
    }

    pub fn attachment_manager_mut(&mut self) -> &mut AttachmentManager {
        &mut self.attachment_manager
    }

    #[cfg(test)]
    pub async fn run_once(&mut self) -> Result<(), Box<dyn Error + Send>> {
        let mut listener = AsyncUnixSocket::new(self.socket_path(), self.auth_group()).unwrap();

        let (sender, mut receiver) = tokio::sync::mpsc::channel(1024);
        let sender = Arc::new(sender);

        tokio::select! {
            accept_result = listener.accept() => {


                match accept_result {
                    Ok((sock, _)) => {
                        let mut socket = listener;
                        let clone = sender.clone();

                        let shutting_down = self.shutting_down;
                        tokio::spawn(async move {
                            if shutting_down {
                                let _ = socket.write("not accepting requests - currently shutting down".as_bytes()).await;
                            } else {
                                handle_client(sock, clone).await;
                            }
                        });
                    },
                    Err(e) => log_error!("Failed to accept connection: {e}"),
                }


            },

            Some((request, mut socket)) = receiver.recv() => {
                let response = handle_request(self, request).await;

                let msg = serde_json::to_string(&response).unwrap();

                tokio::spawn(async move {
                    if let Err(e) = socket.write(msg.as_bytes()).await {
                        log_error!("error sending to socket: {e}");
                    }
                });
            },

            _ = sleep(Duration::from_nanos(1)) => {
                monitor_state(self.processes_mut()).await;

                if  self.shutting_down && self.no_process_running(){
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    pub fn reload(&mut self) -> Result<(), String> {
        let conf = match Config::from_file(self.config_path()) {
            Ok(c) => c,
            Err(e) => return Err(format!("{}", e).to_owned()),
        };
        let mut daemon_new = Daemon::from_config(conf, self.config_path().to_owned());

        let mut leftover = vec![];
        for (name, _p) in self.processes().iter() {
            leftover.push(name.to_owned());
        }

        for (process_name_new, process_new) in daemon_new.processes_mut().drain() {
            match self.processes_mut().get_mut(&process_name_new.to_owned()) {
                Some(process_old) => {
                    if process_old.config() != process_new.config() {

                        process_old.push_desired_state(ProcessState::Stopped);
                    }
                    *process_old.config_mut() = process_new.config().clone();

                    match process_old.config().autostart() {
                        false => process_old.push_desired_state(ProcessState::Idle),
                        true => process_old.push_desired_state(ProcessState::Healthy),
                    }

                    leftover.retain(|n| n != process_old.name());
                }
                None => {
                    let _ = self.processes_mut().insert(process_name_new, process_new);
                }
            }
        }

        for l in leftover.iter() {
            if let Some(p) = self.processes_mut().get_mut(l) {
                p.push_desired_state(ProcessState::Stopped);
            }
        }

        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        unsafe {
            libc::signal(SIGHUP, handler_reload as usize);
        }

        let mut listener = match AsyncUnixSocket::new(self.socket_path(), self.auth_group()) {
            Ok(listener) => listener,
            Err(e) => return Err(Box::<dyn Error>::from(format!("Failed starting the taskmaster daemon: {e}"))),
        };

        let (sender, mut receiver) = tokio::sync::mpsc::channel(1024);
        let sender = Arc::new(sender);
        let mut sigint = signal(SignalKind::interrupt())?;

        loop {
            tokio::select! {
                accept_result = listener.accept() => {

                    match accept_result {
                        Ok((mut sock, _)) => {
                            let clone = sender.clone();

                            let shutting_down = self.shutting_down;

                            tokio::spawn(async move {
                                if shutting_down {
                                    let _ = sock.write("Taskmaster is shutting down - not accepting requests".as_bytes()).await;
                                } else {
                                    handle_client(sock, clone).await;
                                }
                            });
                        },
                        Err(e) => {
                            log_error!("Could not accept connection: {e}");
                            continue;
                        }
                    }

                },

                Some((request, mut socket)) = receiver.recv() => {
                    let response = handle_request(self, request).await;

                    let msg = serde_json::to_string(&response).unwrap();

                    tokio::spawn(async move {
                        if let Err(e) = socket.write(msg.as_bytes()).await {
                            log_error!("error sending to socket: {e}");
                        }
                    });
                },

                _ = sleep(Duration::from_nanos(1)) => {
                    unsafe {

                    #[allow(static_mut_refs)]
                    if RELOAD.load(Ordering::Relaxed) {
                        if let Err(msg) = self.reload() {
                            log_error!("{msg}");
                            return Err(Box::<dyn Error>::from(msg));
                        }
                        RELOAD.store(false, Ordering::Relaxed);
                    }
                    }
                    monitor_state(self.processes_mut()).await;

                    if  self.shutting_down && self.no_process_running(){
                        return Ok(());
                    }
                }
                _ = sigint.recv() => {
                    log_info!("received SIGINT, exiting");
                    self.shutdown();
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

async fn monitor_state(procs: &mut HashMap<String, Process>) {
    for proc in procs.values_mut() {
        proc.desire();
        proc.monitor().await;
    }
}

#[derive(Serialize, Deserialize)]
pub struct MinimumRequest {
    pub id: u32,
}

async fn handle_client(mut socket: UnixStream, sender: Arc<tokio::sync::mpsc::Sender<(Request, UnixStream)>>) {
    let mut line = String::new();

    let (reader_half, mut writer_half) = socket.split();
    let mut reader = BufReader::new(reader_half);

    match reader.read_line(&mut line).await {
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
                            message: format!("{e}").to_owned(),
                            data: None,
                        }),
                    ))
                    .unwrap(),
                    Err(_) => "request id not found - can't respond with JsonRPCError".to_owned(),
                };
                if let Err(e) = writer_half.write_all(error_msg.as_bytes()).await {
                    log_error!("error writing to socket: {e}")
                }
            }
        },
        Err(e) => {
            log_error!("Error reading from socket: {e}");
        }
    }
}

static mut RELOAD: AtomicBool = AtomicBool::new(false);

fn handler_reload() {
    unsafe {
        #[allow(static_mut_refs)]
        RELOAD.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {

    use crate::conf::proc::ProcessConfig;
    use crate::conf::proc::types::{AutoRestart, CommandHealthCheck, HealthCheck, HealthCheckType, UptimeHealthCheck};

    use super::conf::Config;

    use super::*;

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
        let hc = hc.set_check(HealthCheckType::Uptime(UptimeHealthCheck { starttime: 2 }));
        let mut proc = ProcessConfig::default();
        let proc = proc
            .set_cmd("sleep")
            .set_args(vec!["1".to_string()])
            .set_healthcheck(hc.to_owned());
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
        let hc = hc.set_check(HealthCheckType::Uptime(UptimeHealthCheck { starttime: 2 }));
        let mut proc = ProcessConfig::default();
        let proc = proc
            .set_cmd("sh")
            .set_args(vec!["-c".to_string(), "sleep 1 && exit 1".to_string()])
            .set_healthcheck(hc.to_owned());
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
        let hc = hc
            .set_check(HealthCheckType::Uptime(UptimeHealthCheck { starttime: 2 }))
            .set_backoff(1)
            .set_retries(2);
        let mut proc = ProcessConfig::default();
        let proc = proc
            .set_cmd("sh")
            .set_args(vec!["-c".to_string(), "sleep 1 && exit 1".to_string()])
            .set_healthcheck(hc.to_owned());
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
        let hc = hc
            .set_check(HealthCheckType::Uptime(UptimeHealthCheck { starttime: 2 }))
            .set_backoff(1)
            .set_retries(1);
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

        // Run to catch Stopped state (the Stopping -> Stopped transition is done
        // in the same iteration of the state machine).
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Stopped);

        // Run last time to trigger monitor_stopped, which will clear failure counts for eventual
        // future restarts.
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().healthcheck_failures(), 0);

        d.shutdown();
    }

    #[tokio::test]
    async fn healthy_to_failed() {
        let mut hc = HealthCheck::default();
        let hc = hc.set_check(HealthCheckType::Uptime(UptimeHealthCheck { starttime: 1 }));
        let mut proc = ProcessConfig::default();
        let proc = proc
            .set_cmd("sh")
            .set_args(vec!["-c".to_string(), "sleep 2; exit 1".to_string()])
            .set_healthcheck(hc.to_owned());
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
    async fn idle_no_autostart() {
        let mut proc = ProcessConfig::default();
        let proc = proc.set_autostart(false);
        let mut conf = Config::random();
        conf.add_process("process", proc.to_owned());
        let mut d = Daemon::from_config(conf, "path".to_string());

        let _ = d.run_once().await;
        assert_eq!(d.processes().get("process").unwrap().state(), ProcessState::Idle);

        let _ = d.run_once().await;
        assert_eq!(d.processes().get("process").unwrap().state(), ProcessState::Idle);
    }

    #[tokio::test]
    async fn healthy_to_failed_max_retries_reached() {
        let mut hc = HealthCheck::default();
        let hc = hc
            .set_check(HealthCheckType::Uptime(UptimeHealthCheck { starttime: 1 }))
            .set_retries(1)
            .set_backoff(1);
        let mut proc = ProcessConfig::default();
        let proc = proc
            .set_cmd("sh")
            .set_args(vec!["-c".to_string(), "sleep 2; exit 1".to_string()])
            .set_autorestart(AutoRestart {
                mode: "on-failure".to_string(),
                max_retries: Some(1),
            })
            .set_healthcheck(hc.to_owned());
        let mut conf = Config::random();
        let conf = conf.add_process("sleep", proc.clone());
        let mut d = Daemon::from_config(conf.clone(), "idc".to_string());

        let _ = d.run_once().await;

        // Wait for process to be healthy.
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::HealthCheck(_)));
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Healthy);

        // Wait for the process to exit with bad status.
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::Failed(_)));

        // Run again to enter WaitingForRetry state.
        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::WaitingForRetry(_)));
        assert_eq!(d.processes().get("sleep").unwrap().runtime_failures(), 1);

        // Return to healthcheck.
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::HealthCheck(_)));

        tokio::time::sleep(Duration::from_millis(1100)).await;
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Healthy);

        // Wait for the process to exit with bad status.
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::Failed(_)));

        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Stopped);

        // Run again to go into monitor_stopped, which will clear failure counters.
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().runtime_failures(), 0);

        d.shutdown();
    }

    #[tokio::test]
    async fn healthy_to_completed_autorestart() {
        let mut proc = ProcessConfig::default();
        let proc = proc
            .set_cmd("sleep")
            .set_args(vec!["2".to_string()])
            .set_autorestart(AutoRestart {
                mode: "always".to_string(),
                max_retries: None,
            });
        let mut conf = Config::random();
        let conf = conf.add_process("sleep", proc.clone());
        let mut d = Daemon::from_config(conf.clone(), "idc".to_string());

        let _ = d.run_once().await;
        tokio::time::sleep(Duration::from_secs(2)).await;
        let _ = d.run_once().await;

        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Completed);

        // Run again to trigger restart.
        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::HealthCheck(_)));

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
        let _ = d
            .processes_mut()
            .get_mut("sleep")
            .unwrap()
            .push_desired_state(ProcessState::Stopped);
        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::Stopping(_)));

        // Wait for `stoptime` and run once to update state.
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Stopped);

        // Assert that the process stays in Stopped state.
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Stopped);

        d.shutdown();
    }

    #[tokio::test]
    async fn healthcheck_to_healthy_command() {
        let mut hc = HealthCheck::default();
        let hc = hc.set_check(HealthCheckType::Command(CommandHealthCheck {
            cmd: "sleep".to_string(),
            args: vec!["1".to_string()],
            timeout: 10,
        }));
        let mut proc = ProcessConfig::default();
        let proc = proc
            .set_cmd("sleep")
            .set_args(vec!["10".to_string()])
            .set_healthcheck(hc.to_owned());
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
        let hc = hc.set_check(HealthCheckType::Command(CommandHealthCheck {
            cmd: "sleep".to_string(),
            args: vec!["2".to_string()],
            timeout: 1,
        }));
        let mut proc = ProcessConfig::default();
        let proc = proc
            .set_cmd("sleep")
            .set_args(vec!["10".to_string()])
            .set_healthcheck(hc.to_owned());
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
        let hc = hc.set_check(HealthCheckType::Command(CommandHealthCheck {
            cmd: "sleep".to_string(),
            args: vec!["asd".to_string()], // Will fail right away.
            timeout: 1,
        }));
        let mut proc = ProcessConfig::default();
        let proc = proc
            .set_cmd("sleep")
            .set_args(vec!["10".to_string()])
            .set_healthcheck(hc.to_owned());
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
        let hc = hc.set_check(HealthCheckType::Command(CommandHealthCheck {
            cmd: "sleep".to_string(),
            args: vec!["10".to_string()],
            timeout: 10,
        }));
        let mut proc = ProcessConfig::default();
        let proc = proc
            .set_cmd("sleep")
            .set_args(vec!["1".to_string()])
            .set_healthcheck(hc.to_owned());
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
    async fn healthy_to_completed() {
        let mut hc = HealthCheck::default();
        let hc = hc.set_check(HealthCheckType::Command(CommandHealthCheck {
            cmd: "sleep".to_string(),
            args: vec!["10".to_string()],
            timeout: 10,
        }));
        let mut proc = ProcessConfig::default();
        let proc = proc
            .set_cmd("sleep")
            .set_args(vec!["1".to_string()])
            .set_healthcheck(hc.to_owned());
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
