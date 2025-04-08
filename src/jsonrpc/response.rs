use serde::{Deserialize, Serialize, Serializer, ser::Error};

#[derive(Serialize, Deserialize)]
pub struct JsonRPCReponse {
    id: u32,
    #[serde(serialize_with = "json_rpc")]
    json_rpc: String,
    #[serde(flatten)]
    response: JsonRPCReponseType,
}

fn json_rpc<S>(json_rpc: &String, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if json_rpc == "2.0" {
        s.serialize_str(&json_rpc)
    } else {
        Err(Error::custom("json_rpc attribute has to be `2.0`"))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JsonRPCReponseType {
    Result(JsonRPCResponseResult),
    Error(JsonRPCResponseError),
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRPCResponseResult {}

#[derive(Serialize, Deserialize)]
pub struct JsonRPCResponseError {
    code: JsonRPCErrorCode,
    message: String,
    data: Option<JsonRPCResponseErrorData>,
}

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

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRPCResponseErrorData {}
