pub use self::error::*;
pub use self::input::*;
pub use self::output::*;

use crate::errno::ENOSYS;
use crate::warn;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

mod error;
mod input;
mod output;

/// Provides PS4 kernel routines for PS4 application and system libraries.
pub struct Syscalls {
    handlers: [Option<Box<dyn Fn(&SysIn) -> Result<SysOut, SysErr> + Send + Sync>>; 678],
}

impl Syscalls {
    pub fn new() -> Self {
        Self {
            handlers: std::array::from_fn(|_| None),
        }
    }

    /// # Panics
    /// If `id` is not a valid number or the syscall with identifier `id` is already registered.
    pub fn register<O: Send + Sync + 'static>(
        &mut self,
        id: u32,
        o: &Arc<O>,
        h: fn(&Arc<O>, &SysIn) -> Result<SysOut, SysErr>,
    ) {
        let o = o.clone();

        assert!(self.handlers[id as usize]
            .replace(Box::new(move |i| h(&o, i)))
            .is_none());
    }

    /// # Safety
    /// This method may treat any [`SysIn::args`] as a pointer (depend on [`SysIn::id`]). There must
    /// be no any variables that need to be dropped on the stack before calling this method.
    pub unsafe fn exec(&self, i: &SysIn, o: &mut SysOut) -> i64 {
        // Beware that we cannot have any variables that need to be dropped before invoke each
        // syscall handler. The reason is because the handler might exit the calling thread without
        // returning from the handler.
        //
        // See https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/kern/init_sysent.c#L36
        // for standard FreeBSD syscalls.
        let h = match self.handlers.get(i.id as usize) {
            Some(v) => match v {
                Some(v) => v,
                None => todo!("syscall {} at {:#x} on {}", i.id, i.offset, i.module),
            },
            None => return ENOSYS.get().into(),
        };

        // Execute the handler.
        let v = match h(i) {
            Ok(v) => v,
            Err(e) => {
                warn!(e, "Syscall {} failed", i.id);
                return e.errno().get().into();
            }
        };

        // Write the output.
        *o = v;
        0
    }
}

impl Debug for Syscalls {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Syscalls").finish()
    }
}
