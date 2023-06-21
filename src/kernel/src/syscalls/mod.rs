use crate::fs::path::VPathBuf;
use kernel_macros::cpu_abi;

/// Provides PS4 kernel routines.
pub struct Syscalls {}

impl Syscalls {
    pub fn new() -> Self {
        Self {}
    }

    #[cpu_abi]
    pub fn exec(&self, i: &Input, o: &mut Output) -> i64 {
        o.rax = 0;
        o.rdx = 0;

        match i.id {
            _ => panic!(
                "Syscall {} is not implemented at {:#018x} on {}.",
                i.id, i.offset, i.module,
            ),
        }
    }

    #[cpu_abi]
    pub fn int44(&self, offset: usize, module: &VPathBuf) -> ! {
        panic!("Interrupt number 0x44 has been executed at {offset:#018x} on {module}.");
    }
}

/// Input of the syscall entry point.
#[repr(C)]
pub struct Input<'a> {
    pub id: u32,
    pub offset: usize,
    pub module: &'a VPathBuf,
    pub args: [usize; 6],
}

/// Outputs of the syscall entry point.
#[repr(C)]
pub struct Output {
    pub rax: usize,
    pub rdx: usize,
}
