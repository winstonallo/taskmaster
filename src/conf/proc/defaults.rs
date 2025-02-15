use super::deserializers;

pub fn dflt_processes() -> u8 {
    1
}

pub fn dflt_umask() -> deserializers::Umask {
    deserializers::Umask::default()
}

pub fn dflt_autostart() -> bool {
    false
}

pub fn dflt_autorestart() -> deserializers::AutoRestart {
    deserializers::AutoRestart::default()
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

pub fn dflt_stopsignals() -> Vec<deserializers::StopSignal> {
    vec![deserializers::StopSignal::SigTerm]
}

pub fn dflt_stoptime() -> u8 {
    5
}

pub fn dflt_stdout() -> deserializers::WritableFile {
    deserializers::WritableFile::default()
}

pub fn dflt_stderr() -> deserializers::WritableFile {
    deserializers::WritableFile::default()
}
