use crate::conf::{self, proc::ProcessConfig};

#[allow(unused)]
#[derive(Debug)]
pub struct Process<'tm> {
    id: Option<u32>,
    conf: &'tm ProcessConfig,
}

impl<'tm> Process<'tm> {
    pub fn from_process_config(conf: &'tm conf::proc::ProcessConfig) -> Self {
        Self { id: None, conf }
    }
}

#[allow(unused)]
impl<'tm> Process<'tm> {
    pub fn id(&self) -> Option<u32> {
        self.id
    }

    pub fn running(&self) -> bool {
        self.id.is_some()
    }

    pub fn config(&self) -> &'tm ProcessConfig {
        self.conf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_no_id() {
        let proc = Process {
            id: None,
            conf: &conf::proc::ProcessConfig::testconfig(),
        };

        assert!(!proc.running())
    }

    #[test]
    fn running_has_id() {
        let proc = Process {
            id: Some(1),
            conf: &conf::proc::ProcessConfig::testconfig(),
        };

        assert!(proc.running())
    }
}
