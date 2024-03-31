use crate::syscalls::Syscalls;

/// Implementation of `sysentvec` structure.
#[derive(Debug)]
pub struct ProcAbi {
    syscalls: Syscalls, // sv_size + sv_table
}

impl ProcAbi {
    pub fn new(syscalls: Syscalls) -> Self {
        Self { syscalls }
    }

    pub fn syscalls(&self) -> &Syscalls {
        &self.syscalls
    }
}
