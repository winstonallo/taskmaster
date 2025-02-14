use crate::conf;

pub struct Logger {
    stderr: String,
    stdout: String,
}

impl Logger {
    pub fn from_process_config(proc: &conf::proc::ProcessConfig) -> Self {
        Self {
            stdout: proc.get_stdout().to_string(),
            stderr: proc.get_stderr().to_string(),
        }
    }
}
