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
