use crate::conf::proc::deserializers::stopsignal::StopSignal;

use super::deserializers::autorestart::AutoRestart;

pub fn dflt_processes() -> u8 {
    1
}

pub fn dflt_umask() -> String {
    String::from("022")
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

pub fn dflt_stdout() -> String {
    String::from("")
}

pub fn dflt_stderr() -> String {
    String::from("")
}
