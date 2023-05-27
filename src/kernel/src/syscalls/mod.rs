use crate::fs::path::VPathBuf;
use kernel_macros::cpu_abi;

/// Provides PS4 kernel routines.
pub struct Syscalls {}

impl Syscalls {
    pub fn new() -> Self {
        Self {}
    }

    #[cpu_abi]
    pub fn int44(&self, offset: usize, module: *mut VPathBuf) -> ! {
        let module = unsafe { Box::from_raw(module) };

        panic!("Interrupt number 0x44 has been executed at {offset:#018x} on {module}.");
    }

    #[cpu_abi]
    pub fn unimplemented(&self, id: u32, offset: usize, module: *mut VPathBuf) -> ! {
        let module = unsafe { Box::from_raw(module) };

        panic!("Syscall {id} is not implemented at {offset:#018x} on {module}.");
    }
}
