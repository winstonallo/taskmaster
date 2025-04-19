use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct HealthCheck {
    cmd: String,
    args: Vec<String>,
    timeout: usize,
    retries: usize,
    killonfailure: bool,
}
