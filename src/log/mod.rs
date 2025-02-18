use core::fmt;
use std::{fmt::Write, sync::Mutex};

pub struct Logger<W: Write + Send> {
    stderr: Mutex<W>,
    stdout: Mutex<W>,
}

impl<W: Write + Send> Logger<W> {
    pub fn new(stdout: W, stderr: W) -> Self {
        Self {
            stdout: Mutex::new(stdout),
            stderr: Mutex::new(stderr),
        }
    }

    pub fn error(&self, args: fmt::Arguments) {
        let mut guard = self.stderr.lock().expect("Mutex lock panicked in another thread");
        let _ = guard.write_str("error: ");
        let _ = guard.write_fmt(args);
        let _ = guard.write_str("\n");
    }

    pub fn info(&self, args: fmt::Arguments) {
        let mut guard = self.stdout.lock().expect("Mutex lock panicked in another thread");
        let _ = guard.write_str("info: ");
        let _ = guard.write_fmt(args);
        let _ = guard.write_str("\n");
    }
}

#[macro_export]
macro_rules! log_error {
    ($logger:expr, $($arg:tt)*) => {
        $logger.error(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($logger:expr, $($arg:tt)*) => {
        $logger.info(format_args!($($arg)*))
    };
}
