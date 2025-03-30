use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::run::{self, statemachine::states::ProcessState};

#[repr(i16)]
#[derive(Debug, Serialize)]
pub enum JsonRPCErrorCode {
    ServerError(i16), // -32000 to -32099
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
    ParseError = -32700,
}

impl<'de> Deserialize<'de> for JsonRPCErrorCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let code = i16::deserialize(deserializer)?;
        match code {
            -32099..-32000 => Ok(JsonRPCErrorCode::ServerError(code)),
            -32600 => Ok(JsonRPCErrorCode::InvalidRequest),
            -32601 => Ok(JsonRPCErrorCode::MethodNotFound),
            -32602 => Ok(JsonRPCErrorCode::InvalidParams),
            -32603 => Ok(JsonRPCErrorCode::InternalError),
            -32700 => Ok(JsonRPCErrorCode::ParseError),
            _ => Err(serde::de::Error::custom(format!("unknown error code: {}", code))),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonRPCRequest {
    pub jsonrpc: String,
    pub id: usize,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRPCResponse {
    pub jsonrpc: String,
    pub id: usize,
    pub result: serde_json::Value,
}

impl JsonRPCResponse {
    pub fn from_json_rpc_request(request: &JsonRPCRequest, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: request.jsonrpc.clone(),
            id: request.id,
            result: result,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRPCErrorData {
    pub code: JsonRPCErrorCode,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRPCError {
    pub jsonrpc: String,
    pub id: usize,
    pub error: JsonRPCErrorData,
}

impl JsonRPCError {
    pub fn from_json_rpc_request(request: &JsonRPCRequest, data: JsonRPCErrorData) -> Self {
        Self {
            jsonrpc: request.jsonrpc.clone(),
            id: request.id,
            error: data,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct JsonRPCRaw {
    pub jsonrpc: String,
    pub id: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRPCErrorData>,
}

#[derive(Debug)]
pub enum JsonRPCMessage {
    Request(JsonRPCRequest),
    Response(JsonRPCResponse),
    Error(JsonRPCError),
}

impl TryFrom<JsonRPCRaw> for JsonRPCMessage {
    type Error = JsonRPCError;

    fn try_from(value: JsonRPCRaw) -> Result<Self, <JsonRPCMessage as TryFrom<JsonRPCRaw>>::Error> {
        if let Some(error) = value.error {
            return Ok(JsonRPCMessage::Error(JsonRPCError {
                jsonrpc: value.jsonrpc,
                id: value.id,
                error,
            }));
        }

        if let Some(result) = value.result {
            return Ok(JsonRPCMessage::Response(JsonRPCResponse {
                jsonrpc: value.jsonrpc,
                id: value.id,
                result,
            }));
        }

        if let Some(method) = value.method {
            return Ok(JsonRPCMessage::Request(JsonRPCRequest {
                jsonrpc: value.jsonrpc,
                id: value.id,
                method,
                params: value.params,
            }));
        }

        // `jsonrpc` and `id` already are required by deserialization.
        Err(JsonRPCError {
            jsonrpc: value.jsonrpc,
            id: value.id,
            error: JsonRPCErrorData {
                code: JsonRPCErrorCode::InvalidRequest,
                message: format!(
                    "invalid JSON-RPC format: id: {:?}, method: {:?}, params: {:?}, result: {:?}, error: {:?}",
                    value.id, value.method, value.params, value.result, value.error
                ),
                data: None,
            },
        })
    }
}

pub fn handle(request: JsonRPCRequest, procs: &mut HashMap<String, run::statemachine::Process<'_>>) -> Result<JsonRPCResponse, JsonRPCError> {
    match request.method.as_str() {
        "start" => handle_start(request, procs),
        "stop" => handle_stop(request, procs),
        "restart" => handle_restart(request, procs),
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
pub fn handle_restart(request: JsonRPCRequest, procs: &mut HashMap<String, run::statemachine::Process<'_>>) -> Result<JsonRPCResponse, JsonRPCError> {
    let wrong_params_json_rpc_error = JsonRPCError::from_json_rpc_request(
        &request,
        JsonRPCErrorData {
            code: JsonRPCErrorCode::InvalidParams,
            message: format!("wrong or no params given | `name`"),
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
        None => Err(wrong_params_json_rpc_error),
        Some(id) => match id.as_str() {
            None => Err(wrong_params_json_rpc_error),
            Some(id) => match procs.get_mut(id) {
                None => Err(wrong_params_json_rpc_error),
                Some(p) => {
                    use ProcessState::*;
                    let tmp = p.state().clone();
                    match tmp {
                        Idle | WaitingForRetry(_) | Failed(_) | Completed | Stopped => {
                            p.update_state(Ready);
                            Ok(JsonRPCResponse::from_json_rpc_request(&request, json!("process dead - starting process")))
                        }
                        Ready | HealthCheck(_) | Healthy => {
                            let _ = p.stop();
                            p.update_state(Ready);
                            Ok(JsonRPCResponse::from_json_rpc_request(&request, json!("process running - stopping and starting process")))
                        }
                    }
                }
            },
        },
    }
}
pub fn handle_stop(request: JsonRPCRequest, procs: &mut HashMap<String, run::statemachine::Process<'_>>) -> Result<JsonRPCResponse, JsonRPCError> {
    let wrong_params_json_rpc_error = JsonRPCError::from_json_rpc_request(
        &request,
        JsonRPCErrorData {
            code: JsonRPCErrorCode::InvalidParams,
            message: format!("wrong or no params given | `name`"),
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
        None => Err(wrong_params_json_rpc_error),
        Some(id) => match id.as_str() {
            None => Err(wrong_params_json_rpc_error),
            Some(id) => match procs.get_mut(id) {
                None => Err(wrong_params_json_rpc_error),
                Some(p) => {
                    use ProcessState::*;
                    let tmp = p.state().clone();
                    match tmp {
                        Idle | WaitingForRetry(_) | Failed(_) | Completed | Stopped => {
                            Ok(JsonRPCResponse::from_json_rpc_request(&request, json!("already not running")))
                        }
                        Ready | HealthCheck(_) | Healthy => {
                            let _ = p.stop();
                            p.update_state(Stopped);
                            Ok(JsonRPCResponse::from_json_rpc_request(&request, json!("process running, killing process...")))
                        }
                    }
                }
            },
        },
    }
}
pub fn handle_start(request: JsonRPCRequest, procs: &mut HashMap<String, run::statemachine::Process<'_>>) -> Result<JsonRPCResponse, JsonRPCError> {
    let wrong_params_json_rpc_error = JsonRPCError::from_json_rpc_request(
        &request,
        JsonRPCErrorData {
            code: JsonRPCErrorCode::InvalidParams,
            message: format!("wrong or no params given | `name`"),
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
        None => Err(wrong_params_json_rpc_error),
        Some(id) => match id.as_str() {
            None => Err(wrong_params_json_rpc_error),
            Some(id) => match procs.get_mut(id) {
                None => Err(wrong_params_json_rpc_error),
                Some(p) => {
                    use ProcessState::*;
                    let tmp = p.state().clone();
                    match tmp {
                        Idle | WaitingForRetry(_) | Failed(_) | Completed | Stopped => {
                            p.update_state(Ready);
                            Ok(JsonRPCResponse::from_json_rpc_request(&request, json!("starting process")))
                        }
                        Ready | HealthCheck(_) | Healthy => Ok(JsonRPCResponse::from_json_rpc_request(&request, json!("already running not starting"))),
                    }
                }
            },
        },
    }
}
