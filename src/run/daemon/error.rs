use std::{error::Error as StdError, fmt, io};

pub enum DaemonError {
    Internal(String),
}

impl fmt::Display for DaemonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DaemonError::Internal(msg) => write!(f, "internal error: {}", msg),
        }
    }
}

impl fmt::Debug for DaemonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}

impl StdError for DaemonError {}

impl From<DaemonError> for io::Error {
    fn from(value: DaemonError) -> Self {
        io::Error::new(io::ErrorKind::Other, value)
    }
}
