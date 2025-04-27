use std::{error::Error as StdError, fmt, io};

pub enum ProcessError {
    Internal(String),
    CouldNotSpawn(String),
    AlreadyRunning,
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessError::Internal(msg) => write!(f, "internal error: {msg}"),
            ProcessError::CouldNotSpawn(msg) => write!(f, "could not spawn child process: {msg}"),
            ProcessError::AlreadyRunning => write!(f, "process is already running"),
        }
    }
}

impl fmt::Debug for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}

impl StdError for ProcessError {}

impl From<ProcessError> for io::Error {
    fn from(value: ProcessError) -> Self {
        io::Error::other(value)
    }
}
