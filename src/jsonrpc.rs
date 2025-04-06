pub mod handlers;

use serde::{Deserialize, Serialize};

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
            result,
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
