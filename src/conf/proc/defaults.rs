use libc::SIGTERM;

use super::types;

pub fn dflt_args() -> Vec<String> {
    vec![]
}

pub fn dflt_processes() -> u8 {
    1
}

pub fn dflt_umask() -> types::Umask {
    types::Umask::default()
}

pub fn dflt_autostart() -> bool {
    false
}

pub fn dflt_autorestart() -> types::AutoRestart {
    types::AutoRestart::default()
}

pub fn dflt_exitcodes() -> Vec<i32> {
    vec![0i32]
}

pub fn dflt_stopsignals() -> Vec<types::StopSignal> {
    vec![types::StopSignal(SIGTERM)]
}

pub fn dflt_stoptime() -> u8 {
    5
}
