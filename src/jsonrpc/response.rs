use serde::{Deserialize, Serialize, Serializer, ser::Error};

use super::{
    request::{Request, RequestType},
    short_process::ShortProcess,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    id: u32,
    #[serde(serialize_with = "json_rpc")]
    json_rpc: String,
    #[serde(flatten)]
    response_type: ResponseType,
}

impl Response {
    pub fn from_request(request: Request, response_type: ResponseType) -> Self {
        Self {
            id: request.id(),
            json_rpc: "2.0".to_owned(),
            response_type,
        }
    }

    pub fn new(id: u32, response_type: ResponseType) -> Self {
        Self {
            id,
            json_rpc: "2.0".to_owned(),
            response_type,
        }
    }

    pub fn response_type(&self) -> &ResponseType {
        &self.response_type
    }

    // Ugly solution
    pub fn set_response_result(&mut self, request_type: &RequestType) -> &Self {
        match &self.response_type {
            ResponseType::Error(_) => {}
            ResponseType::Result(res) => match res {
                ResponseResult::Status(_) | ResponseResult::StatusSingle(_) => {}
                ResponseResult::Start(msg) | ResponseResult::Stop(msg) | ResponseResult::Restart(msg) | ResponseResult::Attach(msg) => match request_type {
                    RequestType::Start(_) => self.response_type = ResponseType::Result(ResponseResult::Start(msg.to_owned())),
                    RequestType::Stop(_) => self.response_type = ResponseType::Result(ResponseResult::Stop(msg.to_owned())),
                    RequestType::Restart(_) => self.response_type = ResponseType::Result(ResponseResult::Restart(msg.to_owned())),
                    RequestType::Attach(_) => self.response_type = ResponseType::Result(ResponseResult::Attach(msg.to_owned())),
                    _ => {}
                },
                ResponseResult::Reload | ResponseResult::Halt => match request_type {
                    RequestType::Reload => self.response_type = ResponseType::Result(ResponseResult::Reload),
                    RequestType::Halt => self.response_type = ResponseType::Result(ResponseResult::Halt),
                    _ => {}
                },
            },
        }
        self
    }
}

fn json_rpc<S>(json_rpc: &String, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if json_rpc == "2.0" {
        s.serialize_str(json_rpc)
    } else {
        Err(Error::custom("json_rpc attribute has to be `2.0`"))
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ResponseType {
    Result(ResponseResult),
    Error(ResponseError),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ResponseResult {
    Status(Vec<ShortProcess>),
    StatusSingle(ShortProcess),
    Start(String),
    Stop(String),
    Restart(String),
    Reload,
    Halt,
    Attach(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseError {
    pub code: ErrorCode,
    pub message: String,
    pub data: Option<ResponseErrorData>,
}

#[repr(i16)]
#[derive(Debug)]
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
            _ => Err(serde::de::Error::custom(format!("unknown error code: {code}"))),
        }
    }
}
impl Serialize for ErrorCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let n: i16 = match self {
            ErrorCode::ServerError(n) => *n,
            ErrorCode::InvalidRequest => -32600,
            ErrorCode::MethodNotFound => -32601,
            ErrorCode::InvalidParams => -32602,
            ErrorCode::InternalError => -32603,
            ErrorCode::ParseError => -32700,
        };
        serializer.serialize_i16(n)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ResponseErrorData {}
