use std::collections::HashMap;

use super::proc;
use crate::conf;

pub struct Daemon<'tm> {
    processes: HashMap<String, proc::Process<'tm>>,
}

impl<'tm> Daemon<'tm> {
    pub fn from_config(conf: &'tm conf::Config) -> Self {
        let procs: HashMap<String, proc::Process<'tm>> = conf
            .get_processes()
            .iter()
            .map(|(proc_name, proc)| (proc_name.clone(), proc::Process::from_process_config(proc)))
            .collect::<HashMap<String, proc::Process<'tm>>>();

        Self { processes: procs }
    }

    pub fn get_processes(&self) -> &HashMap<String, proc::Process<'tm>> {
        &self.processes
    }
}
