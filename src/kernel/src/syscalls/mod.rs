pub use input::*;
pub use output::*;

use self::error::Error;
use crate::errno::EINVAL;
use crate::fs::path::VPathBuf;
use crate::rtld::RuntimeLinker;
use kernel_macros::cpu_abi;

mod error;
mod input;
mod output;

/// Provides PS4 kernel routines.
pub struct Syscalls<'a, 'b: 'a> {
    ld: &'a RuntimeLinker<'b>,
}

impl<'a, 'b: 'a> Syscalls<'a, 'b> {
    pub fn new(ld: &'a RuntimeLinker<'b>) -> Self {
        Self { ld }
    }

    /// # Safety
    /// This method may treat any [`Input::args`] as a pointer (depend on [`Input::id`]).
    #[cpu_abi]
    pub unsafe fn invoke(&self, i: &Input, o: &mut Output) -> i64 {
        // Execute the handler. See
        // https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/kern/init_sysent.c#L36 for
        // standard FreeBSD syscalls.
        let r = match i.id {
            202 => self.sysctl(
                i.args[0].into(),
                i.args[1].try_into().unwrap(),
                i.args[2].into(),
                i.args[3].into(),
                i.args[4].into(),
                i.args[5].into(),
            ),
            599 => self.relocate_process(),
            _ => todo!("syscall {} at {:#018x} on {}", i.id, i.offset, i.module,),
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

    unsafe fn sysctl(
        &self,
        name: *const i32,
        namelen: u32,
        old: *mut (),
        oldlenp: *mut usize,
        new: *const (),
        newlen: usize,
    ) -> Result<Output, Error> {
        // Convert name to a slice.
        if namelen < 2 || namelen > 24 {
            return Err(Error::Raw(EINVAL));
        }

        let name = std::slice::from_raw_parts(name, namelen.try_into().unwrap());

        // Check type.
        match name[0] {
            t => todo!("sysctl {t}"),
        }
    }

    fn relocate_process(&self) -> Result<Output, Error> {
        self.ld.load_needed()?;
        Ok(Output::ZERO)
    }
}
