use serde::{Deserialize, Deserializer, Serialize, de::Error};

#[derive(Serialize, Deserialize)]
pub struct Request {
    id: u32,
    #[serde(deserialize_with = "json_rpc")]
    json_rpc: String,
    #[serde(flatten)]
    request_type: RequestType,
}

impl Request {
    pub fn new(id: u32, request_type: RequestType) -> Self {
        Self {
            id,
            json_rpc: "2.0".to_owned(),
            request_type,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn request_type(&self) -> &RequestType {
        &self.request_type
    }
}

// This function enforces that the json_rpc key has only as a value 2.0
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
        Err(e) => Err(e),
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum RequestType {
    Status,
    StatusSingle(RequestStatusSingle),
    Start(RequestStart),
    Stop(RequestStop),
    Restart(RequestRestart),
    Reload,
    Halt,
}

impl RequestType {
    pub fn new_status() -> Self {
        Self::Status
    }

    pub fn new_status_single(name: &str) -> Self {
        Self::StatusSingle(RequestStatusSingle {
            params: ParamsName { name: name.to_owned() },
        })
    }

    pub fn new_start(name: &str) -> Self {
        Self::Start(RequestStart {
            params: ParamsName { name: name.to_owned() },
        })
    }

    pub fn new_stop(name: &str) -> Self {
        Self::Stop(RequestStop {
            params: ParamsName { name: name.to_owned() },
        })
    }

    pub fn new_restart(name: &str) -> Self {
        Self::Restart(RequestRestart {
            params: ParamsName { name: name.to_owned() },
        })
    }

    pub fn new_reload() -> Self {
        Self::Reload
    }

    pub fn new_halt() -> Self {
        Self::Halt
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ParamsName {
    name: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct RequestStatusSingle {
    params: ParamsName,
}

impl RequestStatusSingle {
    pub fn name(&self) -> &str {
        &self.params.name
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct RequestStart {
    params: ParamsName,
}

impl RequestStart {
    pub fn name(&self) -> &str {
        &self.params.name
    }
}
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct RequestStop {
    params: ParamsName,
}

impl RequestStop {
    pub fn name(&self) -> &str {
        &self.params.name
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct RequestRestart {
    params: ParamsName,
}

impl RequestRestart {
    pub fn name(&self) -> &str {
        &self.params.name
    }
}

mod test {

    #[test]
    fn test_valid_request() {
        let msg = r#"{
            "id": 2,
            "json_rpc": "2.0",
            "method": "status"
        }"#;
        let _request: super::Request = serde_json::from_str(&msg).unwrap();
    }

    #[test]
    fn test_invalid_json_rpc_version() {
        let json = r#"{
            "id": 3,
            "json_rpc": "1.0",
            "method": "status"
        }"#;

        let result = serde_json::from_str::<super::Request>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_params() {
        let json = r#"{
            "id": 3,
            "json_rpc": "2.0",
            "method": "start",
            "params": { "not_name": 42 }
        }"#;

        let result = serde_json::from_str::<super::Request>(json);
        assert!(result.is_err());
    }
}
