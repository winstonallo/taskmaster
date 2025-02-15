use crate::conf::proc::deserializers::stopsignal::StopSignal;

use super::deserializers::{self, autorestart::AutoRestart, path::WritableFile};

pub fn dflt_processes() -> u8 {
    1
}

pub fn dflt_umask() -> deserializers::umask::Umask {
    deserializers::umask::Umask::default()
}

pub fn dflt_autostart() -> bool {
    false
}

pub fn dflt_autorestart() -> AutoRestart {
    AutoRestart::default()
}

pub fn dflt_exitcodes() -> Vec<u8> {
    vec![0u8]
}

pub fn dflt_startretries() -> u8 {
    3
}

pub fn dflt_startttime() -> u16 {
    5
}

pub fn dflt_stopsignals() -> Vec<crate::conf::proc::deserializers::stopsignal::StopSignal> {
    vec![StopSignal::SigTerm]
}

pub fn dflt_stoptime() -> u8 {
    5
}

pub fn dflt_stdout() -> crate::conf::proc::deserializers::path::WritableFile {
    WritableFile::default()
}

pub fn dflt_stderr() -> crate::conf::proc::deserializers::path::WritableFile {
    WritableFile::default()
}
