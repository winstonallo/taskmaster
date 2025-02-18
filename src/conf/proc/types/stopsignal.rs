use libc::{
    c_int, SIGABRT, SIGALRM, SIGBUS, SIGCHLD, SIGCONT, SIGFPE, SIGHUP, SIGILL, SIGINT, SIGIO, SIGKILL, SIGPIPE, SIGPROF, SIGQUIT, SIGSEGV, SIGSTOP, SIGSYS,
    SIGTERM, SIGTRAP, SIGTSTP, SIGTTIN, SIGTTOU, SIGURG, SIGUSR1, SIGUSR2, SIGVTALRM, SIGWINCH, SIGXCPU, SIGXFSZ,
};
use serde::{Deserialize, Deserializer};

/// # `StopSignal`
/// `src/conf/proc/types/stopsignal.rs`
///
/// Implements the `serde::Deserializer` trait for the `stopsignals` field of the configuration.
#[allow(unused)]
#[derive(Debug, Clone, PartialEq)]
pub struct StopSignal(c_int);

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
