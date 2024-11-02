use crate::proc::ProcAbi;

/// Implementation of [`ProcAbi`] for PS4 processes.
pub struct Ps4Abi;

impl ProcAbi for Ps4Abi {
    fn syscall_handler(&self) {
        todo!()
    }
}
