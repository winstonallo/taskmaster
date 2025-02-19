use core::fmt;
use std::{
    io::{stderr, stdout, Write},
    sync::{Mutex, Once},
    time::{SystemTime, UNIX_EPOCH},
};

use libc::{c_char, localtime, strftime, time_t};

struct Logger {
    stderr: Mutex<Box<dyn Write + Send>>,
    stdout: Mutex<Box<dyn Write + Send>>,
}

impl Logger {
    const FORMAT: &[u8] = b"%Y-%m-%d %H:%M:%S\0";

    pub fn new(stdout: Box<dyn Write + Send>, stderr: Box<dyn Write + Send>) -> Self {
        Self {
            stdout: Mutex::new(stdout),
            stderr: Mutex::new(stderr),
        }
    }

    fn get_time_fmt() -> String {
        let raw_time: time_t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as time_t;

        let time_ptr = unsafe { localtime(&raw_time) };
        if time_ptr.is_null() {
            return "unknown time".to_string();
        }

        let mut buf = [0u8; 64];

        let ret = unsafe { strftime(buf.as_mut_ptr() as *mut c_char, buf.len(), Logger::FORMAT.as_ptr() as *mut c_char, time_ptr) };
        if ret == 0 {
            return "unknown time".to_string();
        }

        String::from_utf8_lossy(&buf[..ret as usize]).into_owned()
    }

    pub fn error(&self, args: fmt::Arguments) {
        let mut guard = self.stderr.lock().expect("Mutex lock panicked in another thread");
        let _ = guard.write_all(Logger::get_time_fmt().as_bytes());
        let _ = guard.write_all(b" [error]: ");
        let _ = guard.write_fmt(args);
        let _ = guard.write_all(b"\n");
    }

    pub fn info(&self, args: fmt::Arguments) {
        let mut guard = self.stdout.lock().expect("Mutex lock panicked in another thread");
        let _ = guard.write_all(Logger::get_time_fmt().as_bytes());
        let _ = guard.write_all(b" [info]: ");
        let _ = guard.write_fmt(args);
        let _ = guard.write_all(b"\n");
    }

    pub fn fatal(&self, args: fmt::Arguments) {
        let mut guard = self.stdout.lock().expect("Mutex lock panicked in another thread");
        let _ = guard.write_all(Logger::get_time_fmt().as_bytes());
        let _ = guard.write_all(b" [fatal]: ");
        let _ = guard.write_fmt(args);
        let _ = guard.write_all(b"\n");
        panic!();
    }
}

static mut INSTANCE: Option<Logger> = None;
static INIT: Once = Once::new();

pub fn error(args: fmt::Arguments) {
    get_logger().error(args);
}

pub fn info(args: fmt::Arguments) {
    get_logger().info(args);
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::log::error(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::log::info(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_fatal {
    ($($arg:tt)*) => {
        $crate::log::fatal(format_args!($($arg)*))
    };
}

fn get_logger() -> &'static Logger {
    INIT.call_once(|| {
        let logger = Logger::new(Box::new(stdout()), Box::new(stderr()));
        unsafe {
            INSTANCE = Some(logger);
        }
    });

    #[allow(static_mut_refs)]
    unsafe {
        INSTANCE.as_ref().expect("there should always be one instance of the logger")
    }
}
