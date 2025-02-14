use crate::conf::{self, proc::ProcessConfig};

#[allow(unused)]
#[derive(Debug)]
pub struct Process<'a> {
    id: Option<u32>,
    running: bool,
    conf: &'a ProcessConfig,
}

impl<'a> Process<'a> {
    pub fn from_process_config(conf: &'a conf::proc::ProcessConfig) -> Self {
        Self {
            id: None,
            running: false,
            conf,
        }
    }
}

#[allow(unused)]
impl<'a> Process<'a> {
    pub fn id(&self) -> Option<u32> {
        self.id
    }

    pub fn running(&self) -> bool {
        self.running
    }

    pub fn config(&self) -> &'a ProcessConfig {
        self.conf
    }
}
