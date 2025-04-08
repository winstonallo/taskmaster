use serde::{de::{Error, Expected}, Deserialize, Deserializer, Serialize};

#[derive(Serialize, Deserialize)]
pub struct JsonRPCRequest {
    id: u32,
    #[serde(deserialize_with = "json_rpc")]
    json_rpc: String,
    #[serde(flatten)]
    request: JsonRPCRequestType,
}

// This function enfores that the json_rpc key has only as a value 2.0
fn json_rpc<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    match String::deserialize(deserializer) {
        Ok(s) => {
            if s == "2.0" {
                Ok(s)
            } else {
                Err(D::Error::invalid_value(serde::de::Unexpected::Str(&s), &"2.0"))
            }
        }
        Err(e) => Err(e)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum JsonRPCRequestType {
    Status,
    StatusSingle(JsonRPCRequestStatusSingle),
    Start(JsonRPCRequestStart),
    Stop(JsonRPCRequestStop),
    Restart(JsonRPCRequestRestart),
    Reload,
    Halt,
}

#[derive(Serialize, Deserialize)]
pub struct JsonRPCParamsName {
    name: String,
}

#[derive(Serialize, Deserialize)]
pub struct JsonRPCRequestStatusSingle {
    params: JsonRPCParamsName,
}

impl JsonRPCRequestType {
    pub fn new_status_single(name: String) -> Self {
        Self::StatusSingle(JsonRPCRequestStatusSingle {
            params: JsonRPCParamsName { name },
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct JsonRPCRequestStart {
    params: JsonRPCParamsName,
}

impl JsonRPCRequestType {
    pub fn new_start(name: String) -> Self {
        Self::Start(JsonRPCRequestStart {
            params: JsonRPCParamsName { name },
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct JsonRPCRequestStop {
    params: JsonRPCParamsName,
}

impl JsonRPCRequestType {
    pub fn new_stop(name: String) -> Self {
        Self::Stop(JsonRPCRequestStop {
            params: JsonRPCParamsName { name },
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct JsonRPCRequestRestart {
    params: JsonRPCParamsName,
}

impl JsonRPCRequestType {
    pub fn new_restart(name: String) -> Self {
        Self::Restart(JsonRPCRequestRestart {
            params: JsonRPCParamsName { name },
        })
    }
}