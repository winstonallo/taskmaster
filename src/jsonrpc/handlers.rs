use super::{
    request::{RequestRestart, RequestStart, RequestStop},
    response::ErrorCode,
};
use crate::{
    conf::Config,
    jsonrpc::{
        response::{ResponseResult, ResponseType},
        short_process::ShortProcess,
    },
    run::{daemon::Daemon, proc::Process, statemachine::states::ProcessState},
};
use std::collections::HashMap;

use super::{
    request::{Request, RequestStatusSingle},
    response::{Response, ResponseError},
};

pub fn handle_request(daemon: &mut Daemon, request: Request) -> Response {
    use super::request::RequestType::*;
    let response_type = match request.request_type() {
        Status => handle_request_status(daemon.processes_mut()),
        StatusSingle(request_status_single) => handle_request_status_single(daemon.processes_mut(), request_status_single),
        Start(request_start) => handle_request_start(daemon.processes_mut(), request_start),
        Stop(request_stop) => handle_request_stop(daemon.processes_mut(), request_stop),
        Restart(request_restart) => handle_request_restart(daemon.processes_mut(), request_restart),
        Reload => handle_request_reload(daemon),
        Halt => handle_request_halt(daemon),
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
                message: format!("error while parsing config file: {}", e),
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
