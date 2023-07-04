pub use output::*;

use self::error::Error;
use crate::fs::path::VPathBuf;
use crate::rtld::RuntimeLinker;
use kernel_macros::cpu_abi;

mod error;
mod output;

/// Provides PS4 kernel routines.
pub struct Syscalls<'a, 'b: 'a> {
    ld: &'a RuntimeLinker<'b>,
}

impl<'a, 'b: 'a> Syscalls<'a, 'b> {
    pub fn new(ld: &'a RuntimeLinker<'b>) -> Self {
        Self { ld }
    }

    #[cpu_abi]
    pub fn invoke(&self, i: &Input, o: &mut Output) -> i64 {
        // Execute the handler. See
        // https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/kern/init_sysent.c#L36 for
        // standard FreeBSD syscalls.
        let r = match i.id {
            599 => self.relocate_process(),
            _ => panic!(
                "Syscall {} is not implemented at {:#018x} on {}.",
                i.id, i.offset, i.module,
            ),
        };

        // Convert the result.
        match r {
            Ok(v) => {
                *o = v;
                0
            }
            Err(e) => {
                o.rax = 0;
                o.rdx = 0;
                e.errno().get().into()
            }
        }
    }

    #[cpu_abi]
    pub fn int44(&self, offset: usize, module: &VPathBuf) -> ! {
        panic!("Interrupt number 0x44 has been executed at {offset:#018x} on {module}.");
    }

    fn relocate_process(&self) -> Result<Output, Error> {
        self.ld.load_needed()?;
        Ok(Output::ZERO)
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
