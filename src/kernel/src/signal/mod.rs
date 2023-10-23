pub use self::set::*;

use bitflags::bitflags;
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::num::NonZeroI32;

mod set;

// List of PS4 signals. The value must be the same as PS4 kernel.
pub const SIGHUP: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(1) };
pub const SIGINT: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(2) };
pub const SIGQUIT: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(3) };
pub const SIGILL: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(4) };
pub const SIGTRAP: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(5) };
pub const SIGABRT: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(6) };
pub const SIGEMT: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(7) };
pub const SIGFPE: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(8) };
pub const SIGKILL: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(9) };
pub const SIGBUS: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(10) };
pub const SIGSEGV: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(11) };
pub const SIGSYS: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(12) };
pub const SIGPIPE: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(13) };
pub const SIGALRM: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(14) };
pub const SIGTERM: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(15) };
pub const SIGURG: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(16) };
pub const SIGSTOP: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(17) };
pub const SIGTSTP: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(18) };
pub const SIGCONT: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(19) };
pub const SIGCHLD: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(20) };
pub const SIGTTIN: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(21) };
pub const SIGTTOU: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(22) };
pub const SIGIO: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(23) };
pub const SIGXCPU: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(24) };
pub const SIGXFSZ: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(25) };
pub const SIGVTALRM: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(26) };
pub const SIGPROF: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(27) };
pub const SIGWINCH: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(28) };
pub const SIGINFO: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(29) };
pub const SIGUSR1: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(30) };
pub const SIGUSR2: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(31) };
pub const SIGTHR: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(32) };
pub const SIGNONE: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(128) };
pub const SIG_MAXSIG: i32 = 128;

// List of sigprocmask operations. The value must be the same as PS4 kernel.
pub const SIG_BLOCK: i32 = 1;
pub const SIG_UNBLOCK: i32 = 2;
pub const SIG_SETMASK: i32 = 3;

pub const SIG_IGN: usize = 1;
pub const SIG_DFL: usize = 0;

pub fn strsignal(num: NonZeroI32) -> Cow<'static, str> {
    match num {
        SIGHUP => "SIGHUP".into(),
        SIGINT => "SIGINT".into(),
        SIGQUIT => "SIGQUIT".into(),
        SIGILL => "SIGILL".into(),
        SIGTRAP => "SIGTRAP".into(),
        SIGABRT => "SIGABRT".into(),
        SIGEMT => "SIGEMT".into(),
        SIGFPE => "SIGFPE".into(),
        SIGKILL => "SIGKILL".into(),
        SIGBUS => "SIGBUS".into(),
        SIGSEGV => "SIGSEGV".into(),
        SIGSYS => "SIGSYS".into(),
        SIGPIPE => "SIGPIPE".into(),
        SIGALRM => "SIGALRM".into(),
        SIGTERM => "SIGTERM".into(),
        SIGURG => "SIGURG".into(),
        SIGSTOP => "SIGSTOP".into(),
        SIGTSTP => "SIGTSTP".into(),
        SIGCONT => "SIGCONT".into(),
        SIGCHLD => "SIGCHLD".into(),
        SIGTTIN => "SIGTTIN".into(),
        SIGTTOU => "SIGTTOU".into(),
        SIGIO => "SIGIO".into(),
        SIGXCPU => "SIGXCPU".into(),
        SIGXFSZ => "SIGXFSZ".into(),
        SIGVTALRM => "SIGVTALRM".into(),
        SIGPROF => "SIGPROF".into(),
        SIGWINCH => "SIGWINCH".into(),
        SIGINFO => "SIGINFO".into(),
        SIGUSR1 => "SIGUSR1".into(),
        SIGUSR2 => "SIGUSR2".into(),
        SIGTHR => "SIGTHR".into(),
        SIGNONE => "SIGNONE".into(),
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
