use serde::{Deserialize, Deserializer};

/// # `StopSignal`
/// `src/conf/proc/types/stopsignal.rs`
///
/// Implements the `serde::Deserializer` trait for the `stopsignals` field of the configuration.
#[allow(unused)]
#[derive(Debug, Clone, PartialEq)]
pub enum StopSignal {
    SigHup,
    SigInt,
    SigQuit,
    SigIll,
    SigTrap,
    SigAbrt,
    SigEmt,
    SigFpe,
    SigKill,
    SigBus,
    SigSegv,
    SigSys,
    SigPipe,
    SigAlrm,
    SigTerm,
    SigUrg,
    SigStop,
    SigTstp,
    SigCont,
    SigChld,
    SigTtin,
    SigTtou,
    SigIo,
    SigXcpu,
    SigXfsz,
    SigVtalrm,
    SigProf,
    SigWinch,
    SigInfo,
    SigUsr1,
    SigUsr2,
    SigThr,
}

impl<'de> Deserialize<'de> for StopSignal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "HUP" | "SIGHUP" => Ok(Self::SigHup),
            "INT" | "SIGINT" => Ok(Self::SigInt),
            "QUIT" | "SIGQUIT" => Ok(Self::SigQuit),
            "ILL" | "SIGILL" => Ok(Self::SigIll),
            "TRAP" | "SIGTRAP" => Ok(Self::SigTrap),
            "ABRT" | "SIGABRT" => Ok(Self::SigAbrt),
            "EMT" | "SIGEMT" => Ok(Self::SigEmt),
            "FPE" | "SIGFPE" => Ok(Self::SigFpe),
            "KILL" | "SIGKILL" => Ok(Self::SigKill),
            "BUS" | "SIGBUS" => Ok(Self::SigBus),
            "SEGV" | "SIGSEGV" => Ok(Self::SigSegv),
            "SYS" | "SIGSYS" => Ok(Self::SigSys),
            "PIPE" | "SIGPIPE" => Ok(Self::SigPipe),
            "ALRM" | "SIGALRM" => Ok(Self::SigAlrm),
            "TERM" | "SIGTERM" => Ok(Self::SigTerm),
            "URG" | "SIGURG" => Ok(Self::SigUrg),
            "STOP" | "SIGSTOP" => Ok(Self::SigStop),
            "TSTP" | "SIGTSTP" => Ok(Self::SigTstp),
            "CONT" | "SIGCONT" => Ok(Self::SigCont),
            "CHLD" | "SIGCHLD" => Ok(Self::SigChld),
            "TTIN" | "SIGTTIN" => Ok(Self::SigTtin),
            "TTOU" | "SIGTTOU" => Ok(Self::SigTtou),
            "IO" | "SIGIO" => Ok(Self::SigIo),
            "XCPU" | "SIGXCPU" => Ok(Self::SigXcpu),
            "XFSZ" | "SIGXFSZ" => Ok(Self::SigXfsz),
            "VTALRM" | "SIGVTALRM" => Ok(Self::SigVtalrm),
            "PROF" | "SIGPROF" => Ok(Self::SigProf),
            "WINCH" | "SIGWINCH" => Ok(Self::SigWinch),
            "INFO" | "SIGINFO" => Ok(Self::SigInfo),
            "USR1" | "SIGUSR1" => Ok(Self::SigUsr1),
            "USR2" | "SIGUSR2" => Ok(Self::SigUsr2),
            "THR" | "SIGTHR" => Ok(Self::SigThr),
            _ => Err(serde::de::Error::custom(format!("invalid value found in stopsignals: '{s}'"))),
        }
    }
}
