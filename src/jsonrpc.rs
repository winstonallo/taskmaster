use serde::{Deserialize, Serialize};

#[repr(i16)]
#[derive(Debug, Serialize)]
pub enum ErrorCode {
    ServerError(i16), // -32000 to -32099
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
    ParseError = -32700,
}

impl<'de> Deserialize<'de> for ErrorCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let code = i16::deserialize(deserializer)?;
        match code {
            -32099..-32000 => Ok(ErrorCode::ServerError(code)),
            -32600 => Ok(ErrorCode::InvalidRequest),
            -32601 => Ok(ErrorCode::MethodNotFound),
            -32602 => Ok(ErrorCode::InvalidParams),
            -32603 => Ok(ErrorCode::InternalError),
            -32700 => Ok(ErrorCode::ParseError),
            _ => Err(serde::de::Error::custom(format!("unknown error code: {}", code))),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
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
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRPCError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRPCError {
    code: ErrorCode,
    message: String,
    data: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
pub struct Raw {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRPCError>,
}

pub trait Method {
    fn handle() -> Result<serde_json::Value, JsonRPCError>;
}
