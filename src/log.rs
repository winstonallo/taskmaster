use core::fmt;
use std::{
    collections::BTreeMap,
    io::Write,
    sync::{Mutex, Once},
    time::{SystemTime, UNIX_EPOCH},
};

use libc::{c_char, localtime, strftime, time_t};
use serde_json::{Value, json};

use crate::run::statemachine::states::ProcessState;

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

    pub fn error(&self, message: fmt::Arguments, fields: BTreeMap<String, Value>) {
        let mut log_entry = fields;
        log_entry.insert("timestamp".to_string(), json!(Logger::get_time_fmt()));
        log_entry.insert("level".to_string(), json!("error"));
        log_entry.insert("message".to_string(), json!(format!("{}", message)));

        let json_str = serde_json::to_string(&log_entry).unwrap_or_else(|_| "{}".to_string());

        let mut guard = self.stderr.lock().expect("Mutex lock panicked in another thread");
        let _ = guard.write_all(json_str.as_bytes());
        let _ = guard.write_all(b"\n");
    }

    pub fn info(&self, message: fmt::Arguments, fields: BTreeMap<String, Value>) {
        let mut log_entry = fields;
        log_entry.insert("timestamp".to_string(), json!(Logger::get_time_fmt()));
        log_entry.insert("level".to_string(), json!("info"));
        log_entry.insert("message".to_string(), json!(format!("{}", message)));

        let json_str = serde_json::to_string(&log_entry).unwrap_or_else(|_| "{}".to_string());

        let mut guard = self.stdout.lock().expect("Mutex lock panicked in another thread");
        let _ = guard.write_all(json_str.as_bytes());
        let _ = guard.write_all(b"\n");
    }

    pub fn warning(&self, message: fmt::Arguments, fields: BTreeMap<String, Value>) {
        let mut log_entry = fields;
        log_entry.insert("timestamp".to_string(), json!(Logger::get_time_fmt()));
        log_entry.insert("level".to_string(), json!("warning"));
        log_entry.insert("message".to_string(), json!(format!("{}", message)));

        let json_str = serde_json::to_string(&log_entry).unwrap_or_else(|_| "{}".to_string());

        let mut guard = self.stdout.lock().expect("Mutex lock panicked in another thread");
        let _ = guard.write_all(json_str.as_bytes());
        let _ = guard.write_all(b"\n");
    }

    #[allow(unused)]
    pub fn fatal(&self, message: fmt::Arguments, fields: BTreeMap<String, Value>) {
        let mut log_entry = fields;
        log_entry.insert("timestamp".to_string(), json!(Logger::get_time_fmt()));
        log_entry.insert("level".to_string(), json!("fatal"));
        log_entry.insert("message".to_string(), json!(format!("{}", message)));

        let json_str = serde_json::to_string(&log_entry).unwrap_or_else(|_| "{}".to_string());

        let mut guard = self.stdout.lock().expect("Mutex lock panicked in another thread");
        let _ = guard.write_all(json_str.as_bytes());
        let _ = guard.write_all(b"\n");
        panic!();
    }
}

static mut INSTANCE: Option<Logger> = None;
static INIT: Once = Once::new();

pub fn error(message: fmt::Arguments, fields: BTreeMap<String, Value>) {
    get_logger().error(message, fields);
}

pub fn info(message: fmt::Arguments, fields: BTreeMap<String, Value>) {
    get_logger().info(message, fields);
}

#[allow(unused)]
pub fn fatal(message: fmt::Arguments, fields: BTreeMap<String, Value>) {
    get_logger().fatal(message, fields);
}

pub fn process_info(name: &str, state: &ProcessState, message: fmt::Arguments, mut fields: BTreeMap<String, Value>) {
    fields.insert("process".to_string(), json!(name));
    fields.insert("state".to_string(), json!(state.to_string()));
    get_logger().info(message, fields);
}

pub fn process_warning(name: &str, state: &ProcessState, message: fmt::Arguments, mut fields: BTreeMap<String, Value>) {
    fields.insert("process".to_string(), json!(name));
    fields.insert("state".to_string(), json!(state.to_string()));
    get_logger().warning(message, fields);
}

pub fn process_error(name: &str, state: &ProcessState, message: fmt::Arguments, mut fields: BTreeMap<String, Value>) {
    fields.insert("process".to_string(), json!(name));
    fields.insert("state".to_string(), json!(state.to_string()));
    get_logger().warning(message, fields);
}

#[macro_export]
macro_rules! define_log_macros {
    ($name:ident, $internal:ident, $func:path) => {
        #[macro_export]
        macro_rules! $name {
            ($fmt:expr, $($arg:tt)*) => {{
                $crate::$internal!($fmt, $($arg)*);
            }};

            ($($arg:tt)*) => {{
                let fields = std::collections::BTreeMap::new();
                $func(format_args!($($arg)*), fields);
            }};
        }

        #[macro_export]
        macro_rules! $internal {
            ($fmt:expr,) => {{
                let fields = std::collections::BTreeMap::new();
                $func(format_args!($fmt), fields);
            }};

            ($fmt:expr, $($arg:expr),+ $(,)?) => {{
                let fields = std::collections::BTreeMap::new();
                $func(format_args!($fmt, $($arg),+), fields);
            }};

            ($fmt:expr, $($farg:expr),* $(,)? ; $($key:ident = $value:expr),* $(,)?) => {{
                let mut fields = std::collections::BTreeMap::new();
                $(
                    fields.insert(stringify!($key).to_string(), serde_json::json!($value));
                )*
                $func(format_args!($fmt, $($farg),*), fields);
            }};
        }
    };
}

#[macro_export]
macro_rules! log_error {
    ($fmt:expr, $($arg:tt)*) => {{
        $crate::_log_error_internal!($fmt, $($arg)*);
    }};

    ($($arg:tt)*) => {{
        let fields = std::collections::BTreeMap::new();
        $crate::log::error(format_args!($($arg)*), fields);
    }};
}

#[macro_export]
macro_rules! _log_error_internal {
    ($fmt:expr,) => {{
        let fields = std::collections::BTreeMap::new();
        $crate::log::error(format_args!($fmt), fields);
    }};

    ($fmt:expr, $($arg:expr),+ $(,)?) => {{
        let fields = std::collections::BTreeMap::new();
        $crate::log::error(format_args!($fmt, $($arg),+), fields);
    }};

    ($fmt:expr, $($farg:expr),* $(,)? ; $($key:ident = $value:expr),* $(,)?) => {{
        let mut fields = std::collections::BTreeMap::new();
        $(
            fields.insert(stringify!($key).to_string(), serde_json::json!($value));
        )*
        $crate::log::error(format_args!($fmt, $($farg),*), fields);
    }};
}

#[macro_export]
macro_rules! log_info {
    // One string + any number of arguments
    // $fmt:expr matches any Rust expression
    // $($arg:tt)* matches the entire rest of the token tree
    ($fmt:expr, $($arg:tt)*) => {{
        $crate::_log_info_internal!($fmt, $($arg)*);
    }};

    // Only one string without formatting
    ($($arg:tt)*) => {{
        let fields = std::collections::BTreeMap::new();
        $crate::log::info(format_args!($($arg)*), fields);
    }};
}

#[macro_export]
macro_rules! _log_info_internal {
    // Empty string with trailing comma.
    ($fmt:expr,) => {{
        let fields = std::collections::BTreeMap::new();
        $crate::log::info(format_args!($fmt), fields);
    }};

    // Regular format string
    // +$(,)? matches one or more arguments with an optional trailing comma
    ($fmt:expr, $($arg:expr),+ $(,)?) => {{
        let fields = std::collections::BTreeMap::new();
        $crate::log::info(format_args!($fmt, $($arg),+), fields);
    }};

    // Format string, zero or more arguments, semicolon delimiter and zero or more key value pairs
    // after the semicolon.
    ($fmt:expr, $($farg:expr),* $(,)? ; $($key:ident = $value:expr),* $(,)?) => {{
        let mut fields = std::collections::BTreeMap::new();
        $(
            // Stringify keys to create JSON fields out of them
            fields.insert(stringify!($key).to_string(), serde_json::json!($value));
        )*
        $crate::log::info(format_args!($fmt, $($farg),*), fields);
    }};
}

#[macro_export]
macro_rules! log_warn {
    ($fmt:expr, $($arg:tt)*) => {{
        $crate::_log_warn_internal!($fmt, $($arg)*);
    }};

    ($($arg:tt)*) => {{
        let mut fields = std::collections::BTreeMap::new();
        $crate::log::warning(format_args!($($arg)*), fields);
    }};
}

#[macro_export]
macro_rules! _log_warn_internal {
    ($fmt:expr,) => {{
        let fields = std::collections::BTreeMap::new();
        $crate::log::warning(format_args!($fmt), fields);
    }};

    ($fmt:expr, $($arg:expr),+ $(,)?) => {{
        let fields = std::collections::BTreeMap::new();
        $crate::log::warning(format_args!($fmt, $($arg),+), fields);
    }};

    ($fmt:expr, $($farg:expr),* $(,)? ; $($key:ident = $value:expr),* $(,)?) => {{
        let mut fields = std::collections::BTreeMap::new();
        $(
            fields.insert(stringify!($key).to_string(), serde_json::json!($value));
        )*
        $crate::log::warning(format_args!($fmt, $($farg),*), fields);
    }};
}

#[macro_export]
macro_rules! proc_info {
    ($proc:expr, $fmt:expr, $($arg:tt)*) => {{
        $crate::_proc_info_internal!($proc, $fmt, $($arg)*);
    }};

    ($proc:expr, $($arg:tt)*) => {{
        let fields = std::collections::BTreeMap::new();
        $crate::log::process_info($proc.name(), &$proc.state(), format_args!($($arg)*), fields);
    }};
}

#[macro_export]
macro_rules! _proc_info_internal {
    ($proc:expr, $fmt:expr,) => {{
        let fields = std::collections::BTreeMap::new();
        $crate::log::process_info($proc.name(), &$proc.state(), format_args!($fmt), fields);
    }};

    ($proc:expr, $fmt:expr, $($arg:expr),+ $(,)?) => {{
        let fields = std::collections::BTreeMap::new();
        $crate::log::process_info($proc.name(), &$proc.state(), format_args!($fmt, $($arg),+), fields);
    }};

    ($proc:expr, $fmt:expr, $($farg:expr),* $(,)? ; $($key:ident = $value:expr),* $(,)?) => {{
        let mut fields = std::collections::BTreeMap::new();
        $(
            fields.insert(stringify!($key).to_string(), serde_json::json!($value));
        )*
        $crate::log::process_info($proc.name(), &$proc.state(), format_args!($fmt, $($farg),*), fields);
    }};
}

#[macro_export]
macro_rules! proc_warning {
    ($proc:expr, $fmt:expr, $($arg:tt)*) => {{
        $crate::_proc_warning_internal!($proc, $fmt, $($arg)*);
    }};

    ($proc:expr, $($arg:tt)*) => {{
        let fields = std::collections::BTreeMap::new();
        $crate::log::process_warning($proc.name(), &$proc.state(), format_args!($($arg)*), fields);
    }};
}

#[macro_export]
macro_rules! _proc_warning_internal {
    ($proc:expr, $fmt:expr,) => {{
        let fields = std::collections::BTreeMap::new();
        $crate::log::process_warning($proc.name(), &$proc.state(), format_args!($fmt), fields);
    }};

    ($proc:expr, $fmt:expr, $($arg:expr),+ $(,)?) => {{
        let fields = std::collections::BTreeMap::new();
        $crate::log::process_warning($proc.name(), &$proc.state(), format_args!($fmt, $($arg),+), fields);
    }};

    ($proc:expr, $fmt:expr, $($farg:expr),* $(,)? ; $($key:ident = $value:expr),* $(,)?) => {{
        let mut fields = std::collections::BTreeMap::new();
        $(
            fields.insert(stringify!($key).to_string(), serde_json::json!($value));
        )*
        $crate::log::process_warning($proc.name(), &$proc.state(), format_args!($fmt, $($farg),*), fields);
    }};
}

#[macro_export]
macro_rules! proc_error {
    ($proc:expr, $fmt:expr, $($arg:tt)*) => {{
        $crate::_proc_error_internal!($proc, $fmt, $($arg)*);
    }};

    ($proc:expr, $($arg:tt)*) => {{
        let fields = std::collections::BTreeMap::new();
        $crate::log::process_error($proc.name(), &$proc.state(), format_args!($($arg)*), fields);
    }};
}

#[macro_export]
macro_rules! _proc_error_internal {
    ($proc:expr, $fmt:expr,) => {{
        let fields = std::collections::BTreeMap::new();
        $crate::log::process_error($proc.name(), &$proc.state(), format_args!($fmt), fields);
    }};

    ($proc:expr, $fmt:expr, $($arg:expr),+ $(,)?) => {{
        let fields = std::collections::BTreeMap::new();
        $crate::log::process_error($proc.name(), &$proc.state(), format_args!($fmt, $($arg),+), fields);
    }};

    ($proc:expr, $fmt:expr, $($farg:expr),* $(,)? ; $($key:ident = $value:expr),* $(,)?) => {{
        let mut fields = std::collections::BTreeMap::new();
        $(
            fields.insert(stringify!($key).to_string(), serde_json::json!($value));
        )*
        $crate::log::process_error($proc.name(), &$proc.state(), format_args!($fmt, $($farg),*), fields);
    }};
}

#[allow(static_mut_refs)]
fn get_logger() -> &'static Logger {
    unsafe {
        if INSTANCE.as_ref().is_none() {
            #[cfg(not(test))]
            INIT.call_once(|| {
                use std::io::{stderr, stdout};
                let logger = Logger::new(Box::new(stdout()), Box::new(stderr()));
                INSTANCE = Some(logger);
            });
            #[cfg(test)]
            INIT.call_once(|| {
                use std::io::sink;
                let logger = Logger::new(Box::new(sink()), Box::new(sink()));
                INSTANCE = Some(logger);
            });
        }
    }
    #[allow(static_mut_refs)]
    unsafe {
        INSTANCE.as_ref().expect("there should always be one instance of the logger")
    }
}

pub fn init(logfile: &str) -> Result<(), String> {
    let stdout = std::fs::File::create(logfile).map_err(|e| e.to_string())?;
    let stderr = std::fs::File::create(logfile).map_err(|e| e.to_string())?;

    let logger = Logger::new(Box::new(stdout), Box::new(stderr));
    unsafe {
        INSTANCE = Some(logger);
    }

    log_info!("logger initialized, logfile: {logfile}");

    Ok(())
}
