use core::fmt;

use libc::{
    SIGABRT, SIGALRM, SIGBUS, SIGCHLD, SIGCONT, SIGFPE, SIGHUP, SIGILL, SIGINT, SIGIO, SIGKILL, SIGPIPE, SIGPROF, SIGQUIT, SIGSEGV, SIGSTOP, SIGSYS, SIGTERM,
    SIGTRAP, SIGTSTP, SIGTTIN, SIGTTOU, SIGURG, SIGUSR1, SIGUSR2, SIGVTALRM, SIGWINCH, SIGXCPU, SIGXFSZ, c_int,
};
use serde::{Deserialize, Deserializer, Serialize};

/// # `StopSignal`
/// `src/conf/proc/types/stopsignal.rs`
///
/// Implements the `serde::Deserializer` trait for the `stopsignals` field of the configuration.
#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StopSignal(pub c_int);

impl StopSignal {
    pub fn signal(&self) -> i32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for StopSignal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "HUP" | "SIGHUP" => Ok(StopSignal(SIGHUP)),
            "INT" | "SIGINT" => Ok(StopSignal(SIGINT)),
            "QUIT" | "SIGQUIT" => Ok(StopSignal(SIGQUIT)),
            "ILL" | "SIGILL" => Ok(StopSignal(SIGILL)),
            "TRAP" | "SIGTRAP" => Ok(StopSignal(SIGTRAP)),
            "ABRT" | "SIGABRT" => Ok(StopSignal(SIGABRT)),
            "FPE" | "SIGFPE" => Ok(StopSignal(SIGFPE)),
            "KILL" | "SIGKILL" => Ok(StopSignal(SIGKILL)),
            "BUS" | "SIGBUS" => Ok(StopSignal(SIGBUS)),
            "SEGV" | "SIGSEGV" => Ok(StopSignal(SIGSEGV)),
            "SYS" | "SIGSYS" => Ok(StopSignal(SIGSYS)),
            "PIPE" | "SIGPIPE" => Ok(StopSignal(SIGPIPE)),
            "ALRM" | "SIGALRM" => Ok(StopSignal(SIGALRM)),
            "TERM" | "SIGTERM" => Ok(StopSignal(SIGTERM)),
            "URG" | "SIGURG" => Ok(StopSignal(SIGURG)),
            "STOP" | "SIGSTOP" => Ok(StopSignal(SIGSTOP)),
            "TSTP" | "SIGTSTP" => Ok(StopSignal(SIGTSTP)),
            "CONT" | "SIGCONT" => Ok(StopSignal(SIGCONT)),
            "CHLD" | "SIGCHLD" => Ok(StopSignal(SIGCHLD)),
            "TTIN" | "SIGTTIN" => Ok(StopSignal(SIGTTIN)),
            "TTOU" | "SIGTTOU" => Ok(StopSignal(SIGTTOU)),
            "IO" | "SIGIO" => Ok(StopSignal(SIGIO)),
            "XCPU" | "SIGXCPU" => Ok(StopSignal(SIGXCPU)),
            "XFSZ" | "SIGXFSZ" => Ok(StopSignal(SIGXFSZ)),
            "VTALRM" | "SIGVTALRM" => Ok(StopSignal(SIGVTALRM)),
            "PROF" | "SIGPROF" => Ok(StopSignal(SIGPROF)),
            "WINCH" | "SIGWINCH" => Ok(StopSignal(SIGWINCH)),
            "USR1" | "SIGUSR1" => Ok(StopSignal(SIGUSR1)),
            "USR2" | "SIGUSR2" => Ok(StopSignal(SIGUSR2)),
            _ => Err(serde::de::Error::custom(format!("invalid value found in stopsignals: '{s}'"))),
        }
    }
}

impl Serialize for StopSignal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            StopSignal(SIGHUP) => serializer.serialize_str("SIGHUP"),
            StopSignal(SIGINT) => serializer.serialize_str("SIGINT"),
            StopSignal(SIGQUIT) => serializer.serialize_str("SIGQUIT"),
            StopSignal(SIGILL) => serializer.serialize_str("SIGILL"),
            StopSignal(SIGTRAP) => serializer.serialize_str("SIGTRAP"),
            StopSignal(SIGABRT) => serializer.serialize_str("SIGABRT"),
            StopSignal(SIGFPE) => serializer.serialize_str("SIGFPE"),
            StopSignal(SIGKILL) => serializer.serialize_str("SIGKILL"),
            StopSignal(SIGBUS) => serializer.serialize_str("SIGBUS"),
            StopSignal(SIGSEGV) => serializer.serialize_str("SIGSEGV"),
            StopSignal(SIGSYS) => serializer.serialize_str("SIGSYS"),
            StopSignal(SIGPIPE) => serializer.serialize_str("SIGPIPE"),
            StopSignal(SIGALRM) => serializer.serialize_str("SIGALRM"),
            StopSignal(SIGTERM) => serializer.serialize_str("SIGTERM"),
            StopSignal(SIGURG) => serializer.serialize_str("SIGURG"),
            StopSignal(SIGSTOP) => serializer.serialize_str("SIGSTOP"),
            StopSignal(SIGTSTP) => serializer.serialize_str("SIGTSTP"),
            StopSignal(SIGCONT) => serializer.serialize_str("SIGCONT"),
            StopSignal(SIGCHLD) => serializer.serialize_str("SIGCHLD"),
            StopSignal(SIGTTIN) => serializer.serialize_str("SIGTTIN"),
            StopSignal(SIGTTOU) => serializer.serialize_str("SIGTTOU"),
            StopSignal(SIGIO) => serializer.serialize_str("SIGIO"),
            StopSignal(SIGXCPU) => serializer.serialize_str("SIGXCPU"),
            StopSignal(SIGXFSZ) => serializer.serialize_str("SIGXFSZ"),
            StopSignal(SIGVTALRM) => serializer.serialize_str("SIGVTALRM"),
            StopSignal(SIGPROF) => serializer.serialize_str("SIGPROF"),
            StopSignal(SIGWINCH) => serializer.serialize_str("SIGWINCH"),
            StopSignal(SIGUSR1) => serializer.serialize_str("SIGUSR1"),
            StopSignal(SIGUSR2) => serializer.serialize_str("SIGUSR2"),
            _ => Err(serde::ser::Error::custom(format!("invalid value for StopSignal (this should never happen): {self:?}"))),
        }
    }
}

impl fmt::Display for StopSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StopSignal(SIGHUP) => write!(f, "SIGHUP"),
            StopSignal(SIGINT) => write!(f, "SIGINT"),
            StopSignal(SIGQUIT) => write!(f, "SIGQUIT"),
            StopSignal(SIGILL) => write!(f, "SIGILL"),
            StopSignal(SIGTRAP) => write!(f, "SIGTRAP"),
            StopSignal(SIGABRT) => write!(f, "SIGABRT"),
            StopSignal(SIGFPE) => write!(f, "SIGFPE"),
            StopSignal(SIGKILL) => write!(f, "SIGKILL"),
            StopSignal(SIGBUS) => write!(f, "SIGBUS"),
            StopSignal(SIGSEGV) => write!(f, "SIGSEGV"),
            StopSignal(SIGSYS) => write!(f, "SIGSYS"),
            StopSignal(SIGPIPE) => write!(f, "SIGPIPE"),
            StopSignal(SIGALRM) => write!(f, "SIGALRM"),
            StopSignal(SIGTERM) => write!(f, "SIGTERM"),
            StopSignal(SIGURG) => write!(f, "SIGURG"),
            StopSignal(SIGSTOP) => write!(f, "SIGSTOP"),
            StopSignal(SIGTSTP) => write!(f, "SIGTSTP"),
            StopSignal(SIGCONT) => write!(f, "SIGCONT"),
            StopSignal(SIGCHLD) => write!(f, "SIGCHLD"),
            StopSignal(SIGTTIN) => write!(f, "SIGTTIN"),
            StopSignal(SIGTTOU) => write!(f, "SIGTTOU"),
            StopSignal(SIGIO) => write!(f, "SIGIO"),
            StopSignal(SIGXCPU) => write!(f, "SIGXCPU"),
            StopSignal(SIGXFSZ) => write!(f, "SIGXFSZ"),
            StopSignal(SIGVTALRM) => write!(f, "SIGVTALRM"),
            StopSignal(SIGPROF) => write!(f, "SIGPROF"),
            StopSignal(SIGWINCH) => write!(f, "SIGWINCH"),
            StopSignal(SIGUSR1) => write!(f, "SIGUSR1"),
            StopSignal(SIGUSR2) => write!(f, "SIGUSR2"),
            _ => Err(fmt::Error),
        }
    }
}
