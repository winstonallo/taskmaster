use std::{error::Error as StdError, fmt, io};

pub enum ProcessError {
    Internal(String),
    CouldNotStartUp(String),
    MaxRetriesReached(String),
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessError::Internal(msg) => write!(f, "internal error: {}", msg),
            ProcessError::CouldNotStartUp(msg) => write!(f, "could not start up: {}", msg),
            ProcessError::MaxRetriesReached(msg) => write!(f, "internal error: {}", msg),
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
