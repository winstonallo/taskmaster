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

pub fn dflt_exitcodes() -> Vec<u8> {
    vec![0u8]
}

pub fn dflt_startretries() -> u8 {
    3
}

pub fn dflt_startttime() -> u16 {
    5
}

pub fn dflt_stopsignals() -> Vec<types::StopSignal> {
    vec![types::StopSignal::SigTerm]
}

pub fn dflt_stoptime() -> u8 {
    5
}

pub fn dflt_stdout() -> types::WritableFile {
    types::WritableFile::default()
}

pub fn dflt_stderr() -> types::WritableFile {
    types::WritableFile::default()
}
