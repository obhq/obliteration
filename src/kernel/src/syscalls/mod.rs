use crate::errno::ENOSYS;
use crate::process::VThread;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

pub use self::error::*;
pub use self::input::*;
pub use self::output::*;

mod error;
mod input;
mod output;

/// Provides PS4 kernel routines for PS4 application and system libraries.
pub struct Syscalls([Option<Handler>; 680]);

impl Syscalls {
    pub const fn new() -> Self {
        // Allows us to initialize the array with `None` without having to resort to non-const
        // operations.
        const NONE: Option<Handler> = None;

        Self([NONE; 680])
    }

    /// # Panics
    /// If `id` is not a valid number or the syscall with identifier `id` is already registered.
    pub fn register<C: Send + Sync + 'static>(
        &mut self,
        id: u32,
        cx: &Arc<C>,
        handler: fn(&Arc<C>, &Arc<VThread>, &SysIn) -> Result<SysOut, SysErr>,
    ) {
        let id: usize = id.try_into().unwrap();
        let cx = cx.clone();

        assert!(self.0[id]
            .replace(Box::new(move |td, i| handler(&cx, td, i)))
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
        let id: usize = i.id.try_into().unwrap();
        let handler = match self.0.get(id) {
            Some(Some(v)) => v,
            Some(None) => todo!(
                "syscall {} at {:#x} on {} with args = [{:#x}, {:#x}, {:#x}, {:#x}, {:#x}, {:#x}]",
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

        // Execute the handler.
        let td = VThread::current().expect("syscall invoked outside of a PS4 thread context");
        let v = match handler(&td, i) {
            Ok(v) => v,
            Err(e) => return e.errno().get().into(),
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

type Handler = Box<dyn Fn(&Arc<VThread>, &SysIn) -> Result<SysOut, SysErr> + Send + Sync>;
