use std::{
    error::Error as StdError,
    fmt::{self},
    io,
};

pub enum ProcessError {
    Internal(String),
    StartUp(String),
    MaxRetriesReached(String),
    Status(String),
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessError::Internal(msg) => write!(f, "internal error: {}", msg),
            ProcessError::StartUp(msg) => write!(f, "could not start up: {}", msg),
            ProcessError::MaxRetriesReached(msg) => write!(f, "max retries reached: {}", msg),
            ProcessError::Status(msg) => write!(f, "could not get process status: {}", msg),
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
        io::Error::new(io::ErrorKind::Other, value)
    }
}
