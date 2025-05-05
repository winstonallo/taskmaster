use super::proc::types::WritableFile;

pub fn dflt_socketpath() -> String {
    "/var/run/taskmaster.sock".to_string()
}

pub fn dflt_authgroup() -> String {
    "taskmaster".to_string()
}

pub fn dflt_logfile() -> WritableFile {
    WritableFile::from_path("/tmp/taskmaster.log")
}
