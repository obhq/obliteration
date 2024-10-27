/// Implementation of `sysentvec` structure.
pub trait ProcAbi: Send + Sync {
    fn syscall_handler(&self);
}
