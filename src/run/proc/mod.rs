use std::{
    fs::File,
    process::{Child, Command},
    time,
};

use crate::conf::{self, proc::ProcessConfig};
pub use error::ProcessError;
use libc::umask;
use state::ProcessState;

mod error;
mod state;

#[allow(unused)]
#[derive(Debug)]
pub struct Process<'tm> {
    id: Option<u32>,
    child: Option<Child>,
    conf: &'tm ProcessConfig,
    last_startup: Option<time::Instant>,
    startup_tries: u8,
    state: ProcessState,
}

impl<'tm> Process<'tm> {
    pub fn from_process_config(conf: &'tm conf::proc::ProcessConfig) -> Self {
        Self {
            id: None,
            child: None,
            conf,
            last_startup: None,
            startup_tries: 0,
            state: ProcessState::Idle,
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

    pub fn start(&mut self) -> Result<(), ProcessError> {
        if self.startup_tries == self.config().startretries() {
            return Err(ProcessError::MaxRetriesReached(format!("tried {} times", self.startup_tries)));
        }
        if self.running() {
            return Ok(());
        }

        let current_umask = unsafe { umask(0) };

        unsafe { umask(self.conf.umask()) };

        let stdout_file = File::create(self.conf.stdout()).map_err(|err| ProcessError::Internal(err.to_string()))?;
        let stderr_file = File::create(self.conf.stderr()).map_err(|err| ProcessError::Internal(err.to_string()))?;

        self.child = match Command::new(self.conf.cmd().path())
            .stdout(stdout_file)
            .stderr(stderr_file)
            .args(self.conf.args())
            .current_dir(self.conf.workingdir().path())
            .spawn()
        {
            Ok(child) => Some(child),
            Err(err) => {
                self.startup_tries += 1;
                return Err(ProcessError::CouldNotStartUp(err.to_string()));
            }
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
            state: ProcessState::Idle,
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
            state: ProcessState::Idle,
        };

        assert!(proc.running())
    }
}
