use rand::{Rng, distr::Alphanumeric, rng};
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    net::UnixStream,
};

use super::{
    request::{AttachFile, RequestAttach, RequestRestart, RequestStart, RequestStop},
    response::ErrorCode,
};
use crate::{
    conf::Config,
    jsonrpc::{
        response::{ResponseResult, ResponseType},
        short_process::ShortProcess,
    },
    log_info,
    run::{daemon::Daemon, proc::Process, statemachine::states::ProcessState},
};
use crate::{log_error, run::daemon::socket::AsyncUnixSocket};
use std::{collections::HashMap, error::Error};

use super::{
    request::{Request, RequestStatusSingle},
    response::{Response, ResponseError},
};

pub async fn handle_request(daemon: &mut Daemon, request: Request) -> Response {
    use super::request::RequestType::*;
    let response_type = match request.request_type() {
        Status => handle_request_status(daemon.processes_mut()),
        StatusSingle(request) => handle_request_status_single(daemon.processes_mut(), request),
        Start(request) => handle_request_start(daemon.processes_mut(), request),
        Stop(request) => handle_request_stop(daemon.processes_mut(), request),
        Restart(request) => handle_request_restart(daemon.processes_mut(), request),
        Reload => handle_request_reload(daemon),
        Halt => handle_request_halt(daemon),
        Attach(request) => handle_request_attach(daemon, request).await,
    };

    Response::from_request(request, response_type)
}

fn handle_request_status(processes: &mut HashMap<String, Process>) -> ResponseType {
    let mut short_processes = vec![];
    for p in processes.values() {
        short_processes.push(ShortProcess::from_process(p));
    }

    ResponseType::Result(ResponseResult::Status(short_processes))
}

fn handle_request_status_single(processes: &mut HashMap<String, Process>, request: &RequestStatusSingle) -> ResponseType {
    let process = match processes.get(request.name()) {
        Some(p) => p,
        None => {
            return ResponseType::Error(ResponseError {
                code: ErrorCode::InvalidParams,
                message: format!("no process with name {} found", request.name()),
                data: None,
            });
        }
    };

    ResponseType::Result(ResponseResult::StatusSingle(ShortProcess::from_process(process)))
}

fn handle_request_start(processes: &mut HashMap<String, Process>, request: &RequestStart) -> ResponseType {
    let process = match processes.get_mut(request.name()) {
        Some(p) => p,
        None => {
            return ResponseType::Error(ResponseError {
                code: ErrorCode::InvalidParams,
                message: format!("no process with name {} found", request.name()),
                data: None,
            });
        }
    };

    process.push_desired_state(ProcessState::Healthy);

    use ProcessState::*;
    match process.state() {
        Healthy | HealthCheck(_) => ResponseType::Result(ResponseResult::Start(format!("process with name {} already running", process.name()))),
        _ => ResponseType::Result(ResponseResult::Start(format!("starting process with name {}", process.name()))),
    }
}

fn handle_request_stop(processes: &mut HashMap<String, Process>, request: &RequestStop) -> ResponseType {
    let process = match processes.get_mut(request.name()) {
        Some(p) => p,
        None => {
            return ResponseType::Error(ResponseError {
                code: ErrorCode::InvalidParams,
                message: format!("no process with name {} found", request.name()),
                data: None,
            });
        }
    };

    process.push_desired_state(ProcessState::Idle);

    use ProcessState::*;
    match process.state() {
        Healthy | HealthCheck(_) => ResponseType::Result(ResponseResult::Stop(format!("stopping process with name {}", process.name()))),
        _ => ResponseType::Result(ResponseResult::Stop(format!("process with name {} not running", process.name()))),
    }
}

fn handle_request_restart(processes: &mut HashMap<String, Process>, request: &RequestRestart) -> ResponseType {
    let process = match processes.get_mut(request.name()) {
        Some(p) => p,
        None => {
            return ResponseType::Error(ResponseError {
                code: ErrorCode::InvalidParams,
                message: format!("no process with name {} found", request.name()),
                data: None,
            });
        }
    };

    process.push_desired_state(ProcessState::Ready);

    ResponseType::Result(ResponseResult::Restart(format!("restarting process with name {} ", process.name())))
}

fn handle_request_reload(daemon: &mut Daemon) -> ResponseType {
    let conf = match Config::from_file(daemon.config_path()) {
        Ok(c) => c,
        Err(e) => {
            return ResponseType::Error(ResponseError {
                code: ErrorCode::InternalError,
                message: format!("error while parsing config file: {e}"),
                data: None,
            });
        }
    };

    let mut daemon_new = Daemon::from_config(conf, daemon.config_path().to_owned());

    let mut leftover = vec![];
    for (name, _p) in daemon.processes().iter() {
        leftover.push(name.to_owned());
    }

    for (process_name_new, process_new) in daemon_new.processes_mut().drain() {
        match daemon.processes_mut().get_mut(&process_name_new.to_owned()) {
            Some(process_old) => {
                *process_old.config_mut() = process_new.config().clone();

                match process_old.config().autostart() {
                    false => process_old.push_desired_state(ProcessState::Idle),
                    true => process_old.push_desired_state(ProcessState::Healthy),
                }

                leftover.retain(|n| n != process_old.name());
            }
            None => {
                let _ = daemon.processes_mut().insert(process_name_new, process_new);
            }
        }
    }

    for l in leftover.iter() {
        if let Some(p) = daemon.processes_mut().get_mut(l) {
            p.push_desired_state(ProcessState::Stopped);
        }
    }

    ResponseType::Result(ResponseResult::Reload)
}

fn handle_request_halt(daemon: &mut Daemon) -> ResponseType {
    for (_name, proc) in daemon.processes_mut().iter_mut() {
        proc.push_desired_state(ProcessState::Stopped);
    }
    daemon.shutdown();

    ResponseType::Result(ResponseResult::Halt)
}

async fn update_attach_stream(file: &mut tokio::fs::File, pos: u64, len: u64, listener: &mut UnixStream) -> Result<u64, Box<dyn Error + Send + Sync>> {
    match file.seek(std::io::SeekFrom::Start(pos)).await {
        Ok(_) => {}
        Err(e) => return Err(Box::<dyn Error + Send + Sync>::from(format!("could not seek file: {e}"))),
    };

    if len <= pos {
        return Ok(if len == pos { pos } else { 0 });
    }

    let mut pos = pos;
    let mut buf = Vec::new();

    match file.read_to_end(&mut buf).await {
        Ok(bytes_read) => {
            if bytes_read > 0 {
                pos += bytes_read as u64;
                listener.write_all(&buf).await?;
            }
        }
        Err(e) => return Err(Box::<dyn Error + Send + Sync>::from(format!("could not read file: {e}"))),
    }

    Ok(pos)
}

async fn attach(socketpath: &str, to: &str, authgroup: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut listener =
        AsyncUnixSocket::new(socketpath, &authgroup).map_err(|e| Box::<dyn Error + Send + Sync>::from(format!("could not create new socket stream: {e}")))?;

    let mut file = tokio::fs::File::open(&to)
        .await
        .map_err(|e| Box::<dyn Error + Send + Sync>::from(format!("could not open stdout at path '{to}': {e}")))?;

    let mut pos = 0;

    let (mut sock, _addr) = match listener.accept().await {
        Ok((sock, addr)) => {
            log_info!("client attached, sending data on {socketpath}");
            (sock, addr)
        }
        Err(e) => return Err(Box::<dyn Error + Send + Sync>::from(format!("could not accept client on {socketpath}: {e}"))),
    };

    loop {
        tokio::select! {
            md = file.metadata() => {
                match md {
                    Ok(metadata) => pos = update_attach_stream(&mut file, pos, metadata.len(), &mut sock).await?,
                    Err(e) => return Err(Box::<dyn Error + Send + Sync>::from(format!("could not get metadata for file '{to}': {e}"))),
                }
            }

        }
    }
}

pub struct AttachmentManager {
    tx: tokio::sync::mpsc::Sender<AttachmentRequest>,
}

enum AttachmentRequest {
    New { socketpath: String, to: String, authgroup: String },
}

impl Default for AttachmentManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AttachmentManager {
    pub fn new() -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);

        tokio::spawn(async move {
            while let Some(req) = rx.recv().await {
                match req {
                    AttachmentRequest::New { socketpath, to, authgroup } => {
                        tokio::spawn(async move {
                            if let Err(e) = attach(&socketpath, &to, authgroup).await {
                                if e.to_string().contains("Broken pipe") {
                                    log_info!("connection on {socketpath} closed");
                                } else {
                                    log_error!("attach: {e}");
                                }
                            }
                        });
                    }
                }
            }
        });

        Self { tx }
    }

    pub async fn attach(&self, socketpath: &str, to: &str, authgroup: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.tx
            .send(AttachmentRequest::New {
                socketpath: socketpath.to_owned(),
                to: to.to_owned(),
                authgroup: authgroup.to_owned(),
            })
            .await
            .map_err(Box::<dyn Error + Send + Sync>::from)
    }
}

async fn handle_request_attach(daemon: &mut Daemon, request: &RequestAttach) -> ResponseType {
    let process = match daemon.processes().get(request.name()) {
        Some(p) => p,
        None => {
            return ResponseType::Error(ResponseError {
                code: ErrorCode::InvalidParams,
                message: format!("process {} not found", request.name()),
                data: None,
            });
        }
    };

    let socketpath = format!("/tmp/{}.sock", rng().sample_iter(&Alphanumeric).take(8).map(char::from).collect::<String>());
    let attach_path = match request.to {
        AttachFile::StdOut => process.config().stdout(),
        AttachFile::StdErr => process.config().stderr(),
    };

    if let Err(e) = daemon
        .attachment_manager()
        .attach(&socketpath, attach_path, daemon.auth_group())
        .await
    {
        let message = format!("could not attach to process {}: {e}", request.name());
        log_error!("{message}");
        return ResponseType::Error(ResponseError {
            code: ErrorCode::InternalError,
            message,
            data: None,
        });
    }

    ResponseType::Result(ResponseResult::Attach {
        name: process.name().to_owned(),
        socketpath,
        to: match request.to {
            AttachFile::StdOut => "stdout".to_string(),
            AttachFile::StdErr => "stderr".to_string(),
        },
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        io::Write,
        sync::atomic::AtomicU32,
        time::Duration,
    };

    use crate::{
        conf::{Config, proc::ProcessConfig},
        jsonrpc::{request::RequestType, short_process},
    };
    static ID_COUNTER: AtomicU32 = AtomicU32::new(1);
    use super::*;

    // Returns a 8 bytes random alphanumeric string.
    fn randstring() -> String {
        use rand::{Rng, distr::Alphanumeric};
        rand::rng().sample_iter(&Alphanumeric).take(8).map(char::from).collect()
    }

    #[tokio::test]
    async fn different_requests_same_id() {
        let mut conf = Config::random();
        let mut proc = ProcessConfig::default();
        let conf = conf.add_process("process", proc.set_autostart(false).to_owned());
        let mut d = Daemon::from_config(conf.to_owned(), "path".to_string());

        let _ = d.run_once().await;

        let _ = handle_request(&mut d, Request::new(1, RequestType::new_status()));
        let response = handle_request(&mut d, Request::new(1, RequestType::new_halt())).await;
        assert!(matches!(response.response_type(), ResponseType::Result(_)));
    }

    #[tokio::test]
    async fn halt() {
        let mut conf = Config::random();
        let conf = conf.add_process("process", ProcessConfig::default());
        let mut d = Daemon::from_config(conf.to_owned(), "path".to_string());

        let _ = d.run_once().await;

        handle_request(&mut d, Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_halt())).await;
        assert_eq!(d.shutting_down(), true);
    }

    #[tokio::test]
    async fn stop() {
        let mut conf = Config::random();
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sleep").set_args(vec!["10".to_string()]);
        let conf = conf.add_process("sleep", proc.to_owned());
        let mut d = Daemon::from_config(conf.to_owned(), "path".to_string());

        let _ = d.run_once().await;

        handle_request(&mut d, Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_stop("sleep"))).await;

        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::Stopping(_)));

        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Stopped);
    }

    #[tokio::test]
    async fn stop_nonexisting_process() {
        let mut conf = Config::random();
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sleep").set_args(vec!["10".to_string()]);
        let conf = conf.add_process("sleep", proc.to_owned());
        let mut d = Daemon::from_config(conf.to_owned(), "path".to_string());

        let _ = d.run_once().await;

        let response = handle_request(
            &mut d,
            Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_stop("notaprocess")),
        )
        .await;

        assert!(matches!(response.response_type(), ResponseType::Error(_)));
    }

    #[tokio::test]
    async fn stop_process_not_running() {
        let mut conf = Config::random();
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sleep").set_args(vec!["10".to_string()]).set_autostart(false);
        let conf = conf.add_process("sleep", proc.to_owned());
        let mut d = Daemon::from_config(conf.to_owned(), "path".to_string());

        let _ = d.run_once().await;

        handle_request(&mut d, Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_stop("sleep"))).await;

        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Idle);
    }

    #[tokio::test]
    async fn status() {
        let mut conf = Config::random();
        let mut proc = ProcessConfig::default();
        let conf = conf.add_process("process", proc.set_autostart(false).to_owned());
        let mut d = Daemon::from_config(conf.to_owned(), "path".to_string());

        let _ = d.run_once().await;

        let response = handle_request(&mut d, Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_status())).await;

        match response.response_type() {
            ResponseType::Result(res) => match res {
                ResponseResult::Status(status) => assert_eq!(*status.get(0).unwrap().state(), short_process::State::Idle),
                _ => panic!("received unexpected response: {:?}", res),
            },
            ResponseType::Error(e) => panic!("handle_request returned an error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn status_single() {
        let mut conf = Config::random();
        let mut proc = ProcessConfig::default();
        let conf = conf.add_process("process", proc.set_autostart(false).to_owned());
        let mut d = Daemon::from_config(conf.to_owned(), "path".to_string());

        let _ = d.run_once().await;

        let response = handle_request(
            &mut d,
            Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_status_single("process")),
        )
        .await;
        assert!(matches!(response.response_type(), ResponseType::Result(_)));

        match response.response_type() {
            ResponseType::Result(res) => match res {
                ResponseResult::StatusSingle(status) => assert_eq!(*status.state(), short_process::State::Idle),
                _ => panic!("received unexpected response: {:?}", res),
            },
            ResponseType::Error(e) => panic!("handle_request returned an error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn status_single_error() {
        let mut conf = Config::random();
        let mut proc = ProcessConfig::default();
        let conf = conf.add_process("process", proc.set_autostart(false).to_owned());
        let mut d = Daemon::from_config(conf.to_owned(), "path".to_string());

        let _ = d.run_once().await;

        let response = handle_request(
            &mut d,
            Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_status_single("notaprocess")),
        )
        .await;
        assert!(matches!(response.response_type(), ResponseType::Error(_)));
    }

    #[tokio::test]
    async fn start() {
        let mut conf = Config::random();
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sleep").set_args(vec!["2".to_string()]);
        let conf = conf.add_process("sleep", proc.set_autostart(false).to_owned());
        let mut d = Daemon::from_config(conf.to_owned(), "path".to_string());

        let _ = d.run_once().await;

        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Idle);

        let response =
            handle_request(&mut d, Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_start("sleep"))).await;
        assert!(matches!(response.response_type(), ResponseType::Result(_)));

        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::HealthCheck(_)));
    }

    #[tokio::test]
    async fn reload_change_in_process() {
        let conf = r#"
        [processes.sleep]
        cmd = "/usr/bin/sleep"
        args = ["2"]
        workingdir = "/tmp"
        autostart = true
        "#;
        let path = format!("/tmp/{}.toml", randstring());
        let mut file = File::create(&path).unwrap();
        let _ = File::write(&mut file, conf.as_bytes());
        let mut conf = Config::from_file(&path).unwrap();
        let conf = conf.set_socketpath(&format!("/tmp/{}.sock", randstring()));
        let mut d = Daemon::from_config(conf.to_owned(), path.to_owned());

        let _ = d.run_once().await;

        assert_eq!(d.processes().get("sleep").unwrap().config().autostart(), true);

        let _ = fs::remove_file(&path);

        let changed_conf = r#"
        [processes.sleep]
        cmd = "/usr/bin/sleep"
        args = ["2"]
        workingdir = "/tmp"
        autostart = false
        "#;
        let mut file = File::create(&path).unwrap();
        let _ = File::write(&mut file, changed_conf.as_bytes());

        handle_request(&mut d, Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_reload())).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        let _ = d.run_once().await;

        assert_eq!(d.processes().get("sleep").unwrap().config().autostart(), false);
    }

    #[tokio::test]
    async fn reload_does_not_restart_processes_with_no_changes() {
        let mut conf = Config::from_file("tests/configs/sleep.toml").unwrap();
        let conf = conf.set_socketpath(&format!("/tmp/{}.sock", randstring()));
        let mut d = Daemon::from_config(conf.to_owned(), "tests/configs/sleep.toml".to_string());

        let _ = d.run_once().await;

        // Wait for process to be healthy
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Healthy);

        handle_request(&mut d, Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_reload())).await;

        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Healthy);
    }

    #[tokio::test]
    async fn reload_config_file_gone() {
        let conf = r#"
        socketpath = "/tmp/.taskmaster.sock"
        authgroup = "winstonallo"

        [processes.sleep]
        cmd = "/usr/bin/sleep"
        args = ["2"]
        processes = 1
        umask = "022"
        workingdir = "/tmp"
        autostart = true
        autorestart = "on-failure[:5]"
        exitcodes = [0, 2]
        stopsignals = ["TERM", "USR1"]
        stoptime = 5
        stdout = "/tmp/sleep.stdout"
        stderr = "/tmp/sleep.stderr"
        env = [["STARTED_BY", "abied-ch"], ["ANSWER", "42"]]
        [processes.sleep.healthcheck]
        starttime = 1
        retries = 3
        backoff = 5
        "#;
        let path = format!("/tmp/{}.toml", randstring());
        let mut file = File::create(&path).unwrap();
        let _ = File::write(&mut file, conf.as_bytes());
        let mut conf = Config::from_file(&path).unwrap();
        let conf = conf.set_socketpath(&format!("/tmp/{}.sock", randstring()));
        let mut d = Daemon::from_config(conf.to_owned(), "path".to_string());

        let _ = d.run_once().await;

        let _ = fs::remove_file(&path);

        let response = handle_request(&mut d, Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_reload())).await;

        assert!(matches!(response.response_type(), ResponseType::Error(_)));
    }

    #[tokio::test]
    async fn restart() {
        let mut conf = Config::random();
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sleep").set_args(vec!["2".to_string()]);
        let conf = conf.add_process("sleep", proc.to_owned());
        let mut d = Daemon::from_config(conf.to_owned(), "path".to_string());

        let _ = d.run_once().await;

        // Wait for process to be healthy
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Healthy);

        handle_request(&mut d, Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_restart("sleep"))).await;
        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::Stopping(_)));

        tokio::time::sleep(Duration::from_millis(100)).await;

        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Stopped);

        let _ = d.run_once().await;
        assert!(matches!(d.processes().get("sleep").unwrap().state(), ProcessState::HealthCheck(_)));
    }

    #[tokio::test]
    async fn restart_nonexisting_process() {
        let mut conf = Config::random();
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sleep").set_args(vec!["2".to_string()]);
        let conf = conf.add_process("sleep", proc.to_owned());
        let mut d = Daemon::from_config(conf.to_owned(), "path".to_string());

        let _ = d.run_once().await;

        // Wait for process to be healthy
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Healthy);

        let response = handle_request(
            &mut d,
            Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_restart("notaprocess")),
        )
        .await;
        assert!(matches!(response.response_type(), ResponseType::Error(_)));
    }

    #[tokio::test]
    async fn start_already_running() {
        let mut conf = Config::random();
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sleep").set_args(vec!["2".to_string()]);
        let conf = conf.add_process("sleep", proc.to_owned());
        let mut d = Daemon::from_config(conf.to_owned(), "path".to_string());

        let _ = d.run_once().await;

        // Wait for process to be healthy
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Healthy);

        // Running start on a running process should have no effect.
        handle_request(&mut d, Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_start("sleep"))).await;
        let _ = d.run_once().await;
        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Healthy);
    }

    #[tokio::test]
    async fn start_error_nonexisting_process() {
        let mut conf = Config::random();
        let mut proc = ProcessConfig::default();
        let proc = proc.set_cmd("sleep").set_args(vec!["2".to_string()]);
        let conf = conf.add_process("sleep", proc.set_autostart(false).to_owned());
        let mut d = Daemon::from_config(conf.to_owned(), "path".to_string());

        let _ = d.run_once().await;

        assert_eq!(d.processes().get("sleep").unwrap().state(), ProcessState::Idle);

        let response = handle_request(
            &mut d,
            Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_start("notaprocess")),
        )
        .await;
        assert!(matches!(response.response_type(), ResponseType::Error(_)));
    }
}
