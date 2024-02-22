use crate::errno::ENOSYS;
use crate::process::VThread;
use crate::warn;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

pub use self::error::*;
pub use self::input::*;
pub use self::output::*;

mod error;
mod input;
mod output;

/// Provides PS4 kernel routines for PS4 application and system libraries.
pub struct Syscalls {
    handlers: [Option<SyscallHandler>; 678],
}

type SyscallHandler = Box<dyn Fn(&VThread, &SysIn) -> Result<SysOut, SysErr> + Send + Sync>;

impl Syscalls {
    pub fn new() -> Self {
        // Allows us to initialize the array with `None` without having to resort to non-const operations.
        const NONE: Option<SyscallHandler> = None;

        Self {
            handlers: [NONE; 678],
        }
    }

    /// # Panics
    /// If `id` is not a valid number or the syscall with identifier `id` is already registered.
    pub fn register<O: Send + Sync + 'static>(
        &mut self,
        id: u32,
        o: &Arc<O>,
        handler: fn(&Arc<O>, &VThread, &SysIn) -> Result<SysOut, SysErr>,
    ) {
        let o = o.clone();

        assert!(self.handlers[id as usize]
            .replace(Box::new(move |td, i| handler(&o, td, i)))
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
        let handler = match self.handlers.get(i.id as usize) {
            Some(Some(v)) => v,
            Some(None) => todo!(
                "syscall {} at {:#x} on {} with args = ({:#x}, {:#x}, {:#x}, {:#x}, {:#x}, {:#x})",
                i.id,
                i.offset,
                i.module,
                i.args[0],
                i.args[1],
                i.args[2],
                i.args[3],
                i.args[4],
                i.args[5],
            ),
            None => return ENOSYS.get().into(),
        };

        let td = VThread::current().expect("Syscall invoked outside of a PS4 thread context");

        // Execute the handler.
        let v = match handler(&td, i) {
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
