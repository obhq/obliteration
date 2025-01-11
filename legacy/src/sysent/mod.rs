use crate::syscalls::Syscalls;
use std::sync::Arc;

/// Implementation of `sysentvec` structure.
#[derive(Debug)]
pub struct ProcAbi {
    syscalls: Option<Arc<Syscalls>>, // sv_size + sv_table
}

impl ProcAbi {
    pub fn new(syscalls: Option<Arc<Syscalls>>) -> Self {
        Self { syscalls }
    }

    pub fn syscalls(&self) -> Option<&Arc<Syscalls>> {
        self.syscalls.as_ref()
    }
}
