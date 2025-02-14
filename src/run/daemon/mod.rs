use std::collections::HashMap;

use super::proc;
use crate::conf;

pub struct Daemon<'a> {
    processes: HashMap<String, proc::Process<'a>>,
}

impl<'a> Daemon<'a> {
    pub fn from_config(conf: &'a conf::Config) -> Self {
        let procs: HashMap<String, proc::Process<'a>> = conf
            .get_processes()
            .iter()
            .map(|(proc_name, proc)| (proc_name.clone(), proc::Process::from_process_config(proc)))
            .collect::<HashMap<String, proc::Process<'a>>>();

        Self { processes: procs }
    }

    pub fn get_processes(&self) -> &HashMap<String, proc::Process<'a>> {
        &self.processes
    }
}
