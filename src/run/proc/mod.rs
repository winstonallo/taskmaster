use std::{
    fs::File,
    process::{Child, Command},
    time,
};

use crate::conf::{self, proc::ProcessConfig};
use libc::umask;

#[allow(unused)]
#[derive(Debug)]
pub struct Process<'tm> {
    id: Option<u32>,
    child: Option<Child>,
    conf: &'tm ProcessConfig,
    last_startup: Option<time::Instant>,
    startup_tries: u8,
}

impl<'tm> Process<'tm> {
    pub fn from_process_config(conf: &'tm conf::proc::ProcessConfig) -> Self {
        Self {
            id: None,
            child: None,
            conf,
            last_startup: None,
            startup_tries: 0,
        }
    }
}

#[allow(unused)]
impl<'tm> Process<'tm> {
    pub fn id(&self) -> Option<u32> {
        self.id
    }

    pub fn running(&self) -> bool {
        self.id.is_some() && self.child.is_some()
    }

    pub fn config(&self) -> &'tm ProcessConfig {
        self.conf
    }

    pub fn last_startup(&self) -> Option<time::Instant> {
        self.last_startup
    }

    pub fn start(&mut self) -> std::io::Result<()> {
        if self.running() {
            return Ok(());
        }

        let current_umask = unsafe { umask(0) };

        unsafe { umask(self.conf.umask()) };

        let stdout_file = File::create(self.conf.stdout())?;
        let stderr_file = File::create(self.conf.stderr())?;

        self.child = match Command::new(self.conf.cmd().path())
            .stdout(stdout_file)
            .stderr(stderr_file)
            .args(self.conf.args())
            .current_dir(self.conf.workingdir().path())
            .spawn()
        {
            Ok(child) => Some(child),
            Err(err) => return Err(err),
        };

        self.id = Some(self.child.as_ref().unwrap().id());
        self.last_startup = Some(time::Instant::now());

        println!("process id: {}", self.id.unwrap());

        Ok(())
    }

    pub fn stop(&mut self) -> std::io::Result<()> {
        if !self.running() {
            return Ok(());
        }

        self.child.take().unwrap().kill();
        self.id.take();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::process::Stdio;

    use super::*;

    #[test]
    fn running_no_id() {
        let proc = Process {
            id: None,
            child: None,
            conf: &conf::proc::ProcessConfig::testconfig(),
            last_startup: None,
            startup_tries: 0,
        };

        assert!(!proc.running())
    }

    #[test]
    fn running_has_id() {
        let proc = Process {
            id: Some(1),
            child: Some(Command::new("/bin/ls").stdout(Stdio::null()).spawn().expect("could not run command")),
            conf: &conf::proc::ProcessConfig::testconfig(),
            last_startup: None,
            startup_tries: 0,
        };

        assert!(proc.running())
    }
}
