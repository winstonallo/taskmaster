use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum JsonRPCRequest {
    Status(JsonRPCRequestStatus),
    StatusSingle(JsonRPCRequestStatusSingle),
    Start(JsonRPCRequestStart),
    Stop(JsonRPCRequestStop),
    Restart(JsonRPCRequestRestart),
    Reload(JsonRPCRequestReload),
    Halt(JsonRPCRequestHalt),
}

#[derive(Serialize, Deserialize)]
pub struct JsonRPCParamsName {
    name: String,
}


#[derive(Serialize, Deserialize)]
pub struct JsonRPCRequestStatus {
    id: u32,
}

impl JsonRPCRequest {
    pub fn new_status(id: u32) -> Self {
        Self::Status(JsonRPCRequestStatus { id })
    }
}


#[derive(Serialize, Deserialize)]
pub struct JsonRPCRequestStatusSingle {
    id: u32,
    params: JsonRPCParamsName,
}

impl JsonRPCRequest {
    pub fn new_status_single(id: u32, name: String) -> Self {
        Self::StatusSingle(JsonRPCRequestStatusSingle {
            id,
            params: JsonRPCParamsName { name },
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct JsonRPCRequestStart {
    id: u32,
    params: JsonRPCParamsName,
}

impl JsonRPCRequest {
    pub fn new_start(id: u32, name: String) -> Self {
        Self::Start(JsonRPCRequestStart {
            id,
            params: JsonRPCParamsName { name },
        })
    }
}


#[derive(Serialize, Deserialize)]
pub struct JsonRPCRequestStop {
    id: u32,
    params: JsonRPCParamsName,
}

impl JsonRPCRequest {
    pub fn new_stop(id: u32, name: String) -> Self {
        Self::Stop(JsonRPCRequestStop {
            id,
            params: JsonRPCParamsName { name },
        })
    }
}


#[derive(Serialize, Deserialize)]
pub struct JsonRPCRequestRestart {
    id: u32,
    params: JsonRPCParamsName,
}

impl JsonRPCRequest {
    pub fn new_restart(id: u32, name: String) -> Self {
        Self::Restart(JsonRPCRequestRestart {
            id,
            params: JsonRPCParamsName { name },
        })
    }
}


#[derive(Serialize, Deserialize)]
pub struct JsonRPCRequestReload {
    id: u32,
}

impl JsonRPCRequest {
    pub fn new_reload(id: u32) -> Self {
        Self::Status(JsonRPCRequestStatus { id })
    }
}


#[derive(Serialize, Deserialize)]
pub struct JsonRPCRequestHalt {
    id: u32,
}

impl JsonRPCRequest {
    pub fn new_halt(id: u32) -> Self {
        Self::Status(JsonRPCRequestStatus { id })
    }
}