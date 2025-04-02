use std::{collections::HashMap, vec};

use super::JsonRPCRequestParams;
use crate::{
    conf::Config,
    run::{
        self,
        daemon::Daemon,
        statemachine::{
            Process,
            states::{HealthCheck, Healthy, ProcessState, Stopped},
        },
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{JsonRPCError, JsonRPCErrorCode, JsonRPCErrorData, JsonRPCRequest, JsonRPCResponse, ResponseProcessState, ResponseProcessStatus};

pub fn handle(request: JsonRPCRequest, procs: &mut HashMap<String, run::statemachine::Process>) -> Result<JsonRPCResponse, JsonRPCError> {
    match request.method.as_str() {
        "start" => handle_start(request, procs),
        "stop" => handle_stop(request, procs),
        "restart" => handle_restart(request, procs),
        "reload" => handle_reload(request, procs),
        "status" => handle_status(request, procs),
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
    if let JsonRPCRequestParams::Halt = request.params.clone() {
        let result = super::JsonRPCResponseResult::Halt;

        let response: JsonRPCResponse = JsonRPCResponse::from_json_rpc_esponse_result(request.clone(), result);

        return Some(response);
    }
    None
    //     JsonRPCRequestParams::Start(name) => match procs.get_mut(&name) {
    // match request.method.as_str() {
    //     "halt" => Some(JsonRPCResponse::from_json_rpc_request(request, json!("taskmaster shutting down - goodbye"))),
    //     _ => None,
    // }
}

// pub fn handle_reload(request: JsonRPCRequest, procs: &mut HashMap<String, run::statemachine::Process>) -> Result<JsonRPCResponse, JsonRPCError> {
//     let wrong_params_json_rpc_error = JsonRPCError::from_json_rpc_request(
//         &request,
//         JsonRPCErrorData {
//             code: JsonRPCErrorCode::InvalidParams,
//             message: "you provided wrong params".to_string(),
//             data: request.params.clone(),
//         },
//     );
//     let conf = match Config::from_file("./config/example.toml") {
//         Ok(c) => c,
//         Err(_e) => return Err(wrong_params_json_rpc_error),
//     };

//     let daemon = match Daemon::from_config(&conf) {
//         Ok(d) => d,
//         Err(_e) => return Err(wrong_params_json_rpc_error),
//     };

//     let mut leftover = vec![];
//     for (name, _p) in procs.iter() {
//         leftover.push(name.to_owned());
//     }

//     for (name, p) in daemon.processes {
//         match procs.get_mut(&name) {
//             Some(old_process) => {
//                 *old_process.config_mut() = p.config().clone();

//                 use ProcessState::*;
//                 match (p.config().autostart(), old_process.state()) {
//                     (false, Ready | HealthCheck(_) | Healthy) => {
//                         let _ = old_process.stop();
//                         old_process.update_state(Idle);
//                     }
//                     (true, Idle | Completed | Stopped | Failed(_) | WaitingForRetry(_)) => {
//                         old_process.update_state(Ready);
//                     }
//                     _ => {}
//                 }
//                 if let Some(index) = leftover.iter().position(|n| *n == old_process.name()) {
//                     leftover.remove(index);
//                 }
//             }
//             None => {
//                 procs.insert(name, p);
//             }
//         }
//     }

//     for l in leftover.iter() {
//         if let Some(mut p) = procs.remove_entry(l) {
//             let _ = p.1.stop();
//         }
//     }

//     Ok(JsonRPCResponse::from_json_rpc_request(&request, json!("sucessfully reloaded config")))
// }

pub fn handle_status(request: JsonRPCRequest, procs: &mut HashMap<String, run::statemachine::Process>) -> Result<JsonRPCResponse, JsonRPCError> {
    match request.params.clone() {
        JsonRPCRequestParams::Status => {
            let mut response_statuses: Vec<ResponseProcessStatus> = vec![];

            for (_, p) in procs.iter() {
                response_statuses.push(ResponseProcessStatus {
                    name: p.name().to_owned(),
                    state: ResponseProcessState::from_process_state(p.state()),
                });
            }

            let result = super::JsonRPCResponseResult::Status(response_statuses);

            let response = JsonRPCResponse::from_json_rpc_esponse_result(request, result);

            Ok(response)
        }
        JsonRPCRequestParams::StatusSingle(name) => match procs.get_mut(&name) {
            None => Err(JsonRPCError::from_json_rpc_request(
                &request,
                JsonRPCErrorData {
                    code: JsonRPCErrorCode::InvalidParams,
                    message: format!("no process for given name - {}", &name),
                    data: Some(request.params.clone()),
                },
            )),
            Some(p) => {
                let response_process_status = ResponseProcessStatus {
                    name: p.name().to_owned(),
                    state: ResponseProcessState::from_process_state(p.state()),
                };
                let result = super::JsonRPCResponseResult::StatusSingle(response_process_status);

                let response: JsonRPCResponse = JsonRPCResponse::from_json_rpc_esponse_result(request, result);

                Ok(response)
            }
        },
        _ => Err(JsonRPCError::from_json_rpc_request(
            &request,
            JsonRPCErrorData {
                code: JsonRPCErrorCode::InvalidParams,
                message: format!("wrong params for method {}", request.method.clone()),
                data: Some(request.params.clone()),
            },
        )),
    }
}

pub fn handle_reload(request: JsonRPCRequest, procs: &mut HashMap<String, run::statemachine::Process>) -> Result<JsonRPCResponse, JsonRPCError> {
    let wrong_params_json_rpc_error = Err(JsonRPCError::from_json_rpc_request(
        &request,
        JsonRPCErrorData {
            code: JsonRPCErrorCode::InvalidParams,
            message: format!("wrong params for method {}", request.method.clone()),
            data: Some(request.params.clone()),
        },
    ));
    match request.params.clone() {
        JsonRPCRequestParams::Reload => {
            let conf = match Config::from_file("./config/felix.toml") {
                Ok(c) => c,
                Err(_e) => return wrong_params_json_rpc_error,
            };

            let daemon = match Daemon::from_config(&conf) {
                Ok(d) => d,
                Err(_e) => return wrong_params_json_rpc_error,
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
                        match (p.config().autostart(), old_process.state()) {
                            (false, Ready | HealthCheck(_) | Healthy) => {
                                let _ = old_process.stop();
                                old_process.update_state(Idle);
                            }
                            (true, Idle | Completed | Stopped | Failed(_) | WaitingForRetry(_)) => {
                                old_process.update_state(Ready);
                            }
                            _ => {}
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
                if let Some(mut p) = procs.remove_entry(l) {
                    let _ = p.1.stop();
                }
            }

            let result = super::JsonRPCResponseResult::Reload;

            let response: JsonRPCResponse = JsonRPCResponse::from_json_rpc_esponse_result(request, result);

            Ok(response)
        }
        _ => Err(JsonRPCError::from_json_rpc_request(
            &request,
            JsonRPCErrorData {
                code: JsonRPCErrorCode::InvalidParams,
                message: format!("wrong params for method {}", request.method.clone()),
                data: Some(request.params.clone()),
            },
        )),
    }
}
// let params = match request.params.clone() {
//     Some(value) => value,
//     None => return Err(wrong_params_json_rpc_error),
// };

// let object = match params.as_object() {
//     Some(object) => object,
//     None => return Err(wrong_params_json_rpc_error),
// };
// match object.get("name") {
//     None => {
//         let resp = StatusResponse { processes: vec![]};

//         let mut line: String = String::new();
//         line.push_str("processes: [");

//         for (_, p) in procs.iter() {
//             if line.len() > "processes: [".len() {
//                 line.push(',');
//             }
//             line.push_str(&format!("{{name: {}, state: {}}}", p.name(), p.state()));
//         }
//         line.push(']');

//         Ok(JsonRPCResponse::from_json_rpc_request(&request, json!(line)))
//     }
//     Some(id) => match id.as_str() {
//         None => Err(wrong_params_json_rpc_error),
//         Some(id) => match procs.get_mut(id) {
//             None => Err(wrong_params_json_rpc_error),
//             Some(p) => Ok(JsonRPCResponse::from_json_rpc_request(
//                 &request,
//                 json!(format!(r#"{{"name": {}, "state": {}}}"#, p.name(), p.state())),
//             )),
//         },
//     },
//     // }
// }

// pub fn handle_restart(request: JsonRPCRequest, procs: &mut HashMap<String, run::statemachine::Process>) -> Result<JsonRPCResponse, JsonRPCError> {
//     match get_proc_from_json_request(&request, procs) {
//         Ok(proc) => {
//             use ProcessState::*;
//             match proc.state().clone() {
//                 Idle | WaitingForRetry(_) | Failed(_) | Completed | Stopped => {
//                     proc.update_state(Ready);
//                     Ok(JsonRPCResponse::from_json_rpc_request(&request, json!("process dead - starting process")))
//                 }
//                 Ready | HealthCheck(_) | Healthy => {
//                     let _ = proc.stop();
//                     proc.update_state(Ready);
//                     Ok(JsonRPCResponse::from_json_rpc_request(
//                         &request,
//                         json!("process running - stopping and starting process"),
//                     ))
//                 }
//             }
//         }
//         Err(e) => Err(e),
//     }
// }

pub fn handle_restart(request: JsonRPCRequest, procs: &mut HashMap<String, run::statemachine::Process>) -> Result<JsonRPCResponse, JsonRPCError> {
    match request.params.clone() {
        JsonRPCRequestParams::Restart(name) => match procs.get_mut(&name) {
            None => Err(JsonRPCError::from_json_rpc_request(
                &request,
                JsonRPCErrorData {
                    code: JsonRPCErrorCode::InvalidParams,
                    message: format!("no process for given name - {}", &name),
                    data: Some(request.params.clone()),
                },
            )),
            Some(p) => {
                use ProcessState::*;
                match p.state() {
                    Healthy | HealthCheck(_) => {
                        let _ = p.stop();
                        p.update_state(Ready);
                    }
                    _ => {}
                }

                p.update_state(ProcessState::Ready);

                let result = super::JsonRPCResponseResult::Restart(name);

                let response: JsonRPCResponse = JsonRPCResponse::from_json_rpc_esponse_result(request, result);

                Ok(response)
            }
        },
        _ => Err(JsonRPCError::from_json_rpc_request(
            &request,
            JsonRPCErrorData {
                code: JsonRPCErrorCode::InvalidParams,
                message: format!("wrong params for method {}", request.method.clone()),
                data: Some(request.params.clone()),
            },
        )),
    }
}
pub fn handle_stop(request: JsonRPCRequest, procs: &mut HashMap<String, run::statemachine::Process>) -> Result<JsonRPCResponse, JsonRPCError> {
    match request.params.clone() {
        JsonRPCRequestParams::Stop(name) => match procs.get_mut(&name) {
            None => Err(JsonRPCError::from_json_rpc_request(
                &request,
                JsonRPCErrorData {
                    code: JsonRPCErrorCode::InvalidParams,
                    message: format!("no process for given name - {}", &name),
                    data: Some(request.params.clone()),
                },
            )),
            Some(p) => {
                if let Err(_e) = p.stop() {
                    return Err(JsonRPCError::from_json_rpc_request(
                        &request,
                        JsonRPCErrorData {
                            code: JsonRPCErrorCode::InvalidParams,
                            message: format!("process `{}` not running ", p.name()),
                            data: Some(request.params.clone()),
                        },
                    ));
                }

                p.update_state(ProcessState::Stopped);

                let result = super::JsonRPCResponseResult::Stop(name);

                let response: JsonRPCResponse = JsonRPCResponse::from_json_rpc_esponse_result(request, result);

                Ok(response)
            }
        },
        _ => Err(JsonRPCError::from_json_rpc_request(
            &request,
            JsonRPCErrorData {
                code: JsonRPCErrorCode::InvalidParams,
                message: format!("wrong params for method {}", request.method.clone()),
                data: Some(request.params.clone()),
            },
        )),
    }
}
pub fn handle_start(request: JsonRPCRequest, procs: &mut HashMap<String, run::statemachine::Process>) -> Result<JsonRPCResponse, JsonRPCError> {
    match request.params.clone() {
        JsonRPCRequestParams::Start(name) => match procs.get_mut(&name) {
            None => Err(JsonRPCError::from_json_rpc_request(
                &request,
                JsonRPCErrorData {
                    code: JsonRPCErrorCode::InvalidParams,
                    message: format!("no process for given name - {}", &name),
                    data: Some(request.params.clone()),
                },
            )),
            Some(p) => {
                use ProcessState::*;
                match p.state() {
                    Healthy | HealthCheck(_) => {
                        return Err(JsonRPCError::from_json_rpc_request(
                            &request,
                            JsonRPCErrorData {
                                code: JsonRPCErrorCode::InternalError,
                                message: format!("process with name {} already running", name),
                                data: Some(request.params.clone()),
                            },
                        ));
                    }
                    _ => {
                        p.update_state(ProcessState::Ready);

                        let result = super::JsonRPCResponseResult::Start(name);

                        let response: JsonRPCResponse = JsonRPCResponse::from_json_rpc_esponse_result(request, result);

                        Ok(response)
                    }
                }
            }
        },
        _ => Err(JsonRPCError::from_json_rpc_request(
            &request,
            JsonRPCErrorData {
                code: JsonRPCErrorCode::InvalidParams,
                message: format!("wrong params for method {}", request.method.clone()),
                data: Some(request.params.clone()),
            },
        )),
    }
}
// pub fn handle_start(request: JsonRPCRequest, procs: &mut HashMap<String, run::statemachine::Process>) -> Result<JsonRPCResponse, JsonRPCError> {
//     match get_proc_from_json_request(&request, procs) {
//         Ok(p) => {
//             use ProcessState::*;
//             match p.state().clone() {
//                 Idle | WaitingForRetry(_) | Failed(_) | Completed | Stopped => {
//                     p.update_state(Ready);
//                     Ok(JsonRPCResponse::from_json_rpc_request(&request, json!("starting process")))
//                 }
//                 Ready | HealthCheck(_) | Healthy => Ok(JsonRPCResponse::from_json_rpc_request(&request, json!("already running not starting"))),
//             }
//         }
//         Err(e) => Err(e),
//     }
// }
