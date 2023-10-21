pub use self::set::*;

use bitflags::bitflags;
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::num::NonZeroI32;

mod set;

// List of PS4 signals. The value must be the same as PS4 kernel.
pub const SIGKILL: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(9) };
pub const SIGSTOP: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(17) };
pub const SIGCHLD: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(20) };
pub const SIG_MAXSIG: i32 = 128;

// List of sigprocmask operations. The value must be the same as PS4 kernel.
pub const SIG_BLOCK: i32 = 1;
pub const SIG_UNBLOCK: i32 = 2;
pub const SIG_SETMASK: i32 = 3;

pub const SIG_IGN: usize = 1;
pub const SIG_DFL: usize = 0;

pub fn strsignal(num: NonZeroI32) -> Cow<'static, str> {
    match num {
        SIGKILL => "SIGKILL".into(),
        SIGSTOP => "SIGSTOP".into(),
        SIGCHLD => "SIGCHLD".into(),
        v => format!("{v}").into(),
    }
}

/// An implementation of `sigaction` structure.
#[repr(C)]
pub struct SignalAct {
    pub handler: usize,     // sa_handler
    pub flags: SignalFlags, // sa_flags
    pub mask: SignalSet,    // sa_mask
}

bitflags! {
    /// Flags for [`SignalAct`].
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    pub struct SignalFlags: u32 {
        const SA_ONSTACK = 0x0001;
        const SA_RESTART = 0x0002;
        const SA_RESETHAND = 0x0004;
        const SA_NODEFER = 0x0010;
        const SA_SIGINFO = 0x0040;
    }
}

impl Display for SignalFlags {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
