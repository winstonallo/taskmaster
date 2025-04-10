use std::collections::HashMap;

use serde_json::json;

use crate::{
    conf::Config,
    run::{daemon::Daemon, proc::Process, statemachine::states::ProcessState},
};

use super::{JsonRPCError, JsonRPCErrorCode, JsonRPCErrorData, JsonRPCRequest, JsonRPCResponse};

pub fn handle(request: JsonRPCRequest, procs: &mut HashMap<String, Process>) -> Result<JsonRPCResponse, JsonRPCError> {
    match request.method.as_str() {
        "start" => handle_start(request, procs),
        "stop" => handle_stop(request, procs),
        "restart" => handle_restart(request, procs),
        "status" => handle_status(request, procs),
        "reload" => handle_reload(request, procs),
        _ => Err(JsonRPCError::from_json_rpc_request(
            &request,
            JsonRPCErrorData {
                code: JsonRPCErrorCode::MethodNotFound,
                message: format!("method {} not implemented", request.method),
                data: None,
            },
        )),
    }
}
pub fn handle_halt(request: &JsonRPCRequest) -> Option<JsonRPCResponse> {
    match request.method.as_str() {
        "halt" => Some(JsonRPCResponse::from_json_rpc_request(request, json!("taskmaster shutting down - goodbye"))),
        _ => None,
    }
}
pub fn handle_reload(request: JsonRPCRequest, procs: &mut HashMap<String, Process>) -> Result<JsonRPCResponse, JsonRPCError> {
    let wrong_params_json_rpc_error = JsonRPCError::from_json_rpc_request(
        &request,
        JsonRPCErrorData {
            code: JsonRPCErrorCode::InvalidParams,
            message: "you provided wrong params".to_string(),
            data: request.params.clone(),
        },
    );
    let conf = match Config::from_file("./config/example.toml") {
        Ok(c) => c,
        Err(_e) => return Err(wrong_params_json_rpc_error),
    };

    let daemon = match Daemon::from_config(&conf) {
        Ok(d) => d,
        Err(_e) => return Err(wrong_params_json_rpc_error),
    };

    let mut leftover = vec![];
    for (name, _p) in procs.iter() {
        leftover.push(name.to_owned());
    }

    for (name, p) in daemon.processes {
        match procs.get_mut(&name) {
            Some(old_process) => {
                *old_process.config_mut() = p.config().clone();

                use ProcessState::*;
                match p.config().autostart() {
                    false => old_process.push_desired_state(Idle),
                    true => old_process.push_desired_state(Healthy),
                }

                if let Some(index) = leftover.iter().position(|n| *n == old_process.name()) {
                    leftover.remove(index);
                }
            }
            None => {
                procs.insert(name, p);
            }
        }
    }

    for l in leftover.iter() {
        if let Some(p) = procs.get_mut(l) {
            p.push_desired_state(ProcessState::Stopped);
        }
    }

    Ok(JsonRPCResponse::from_json_rpc_request(&request, json!("sucessfully reloaded config")))
}

pub fn get_proc_from_json_request<'a>(request: &JsonRPCRequest, procs: &'a mut HashMap<String, Process>) -> Result<&'a mut Process, JsonRPCError> {
    let wrong_params_json_rpc_error = JsonRPCError::from_json_rpc_request(
        request,
        JsonRPCErrorData {
            code: JsonRPCErrorCode::InvalidParams,
            message: "wrong or no params given | `name`".to_string(),
            data: request.params.clone(),
        },
    );
    let params = match request.params.clone() {
        Some(value) => value,
        None => return Err(wrong_params_json_rpc_error),
    };

    let object = match params.as_object() {
        Some(object) => object,
        None => return Err(wrong_params_json_rpc_error),
    };

    let name = match object.get("name") {
        Some(name) => name,
        None => return Err(wrong_params_json_rpc_error),
    };

    let name = match name.as_str() {
        Some(name) => name,
        None => return Err(wrong_params_json_rpc_error),
    };

    let proc = match procs.get_mut(&name.to_owned()) {
        None => return Err(wrong_params_json_rpc_error),
        Some(p) => p,
    };

    Ok(proc)
}

pub fn handle_status(request: JsonRPCRequest, procs: &mut HashMap<String, Process>) -> Result<JsonRPCResponse, JsonRPCError> {
    let wrong_params_json_rpc_error = JsonRPCError::from_json_rpc_request(
        &request,
        JsonRPCErrorData {
            code: JsonRPCErrorCode::InvalidParams,
            message: "wrong or no params given | `name`".to_string(),
            data: request.params.clone(),
        },
    );
    let params = match request.params.clone() {
        Some(value) => value,
        None => return Err(wrong_params_json_rpc_error),
    };

    let object = match params.as_object() {
        Some(object) => object,
        None => return Err(wrong_params_json_rpc_error),
    };
    match object.get("name") {
        None => {
            let mut line: String = String::new();
            line.push_str("processes: [");
            for (_, p) in procs.iter() {
                line.push_str(&format!("{{name: {}, state: {}}}", p.name(), p.state()));
            }
            line.push(']');

            Ok(JsonRPCResponse::from_json_rpc_request(&request, json!(line)))
        }
        Some(id) => match id.as_str() {
            None => Err(wrong_params_json_rpc_error),
            Some(id) => match procs.get_mut(id) {
                None => Err(wrong_params_json_rpc_error),
                Some(p) => Ok(JsonRPCResponse::from_json_rpc_request(
                    &request,
                    json!(format!(r#"{{"name": {}, "state": {}}}"#, p.name(), p.state())),
                )),
            },
        },
    }
}

pub fn handle_restart(request: JsonRPCRequest, procs: &mut HashMap<String, Process>) -> Result<JsonRPCResponse, JsonRPCError> {
    match get_proc_from_json_request(&request, procs) {
        Ok(proc) => {
            proc.push_desired_state(ProcessState::Ready);
            Ok(JsonRPCResponse::from_json_rpc_request(&request, json!("restarting process")))
        }
        Err(e) => Err(e),
    }
}

pub fn handle_stop(request: JsonRPCRequest, procs: &mut HashMap<String, Process>) -> Result<JsonRPCResponse, JsonRPCError> {
    match get_proc_from_json_request(&request, procs) {
        Ok(p) => {
            p.push_desired_state(ProcessState::Stopped);
            Ok(JsonRPCResponse::from_json_rpc_request(&request, json!("stopping process")))
        }
        Err(e) => Err(e),
    }
}

pub fn handle_start(request: JsonRPCRequest, procs: &mut HashMap<String, Process>) -> Result<JsonRPCResponse, JsonRPCError> {
    match get_proc_from_json_request(&request, procs) {
        Ok(p) => {
            p.push_desired_state(ProcessState::Healthy);
            Ok(JsonRPCResponse::from_json_rpc_request(&request, json!("starting process")))
        }
        Err(e) => Err(e),
    }
}
