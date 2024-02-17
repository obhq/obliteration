pub use self::set::*;

use bitflags::bitflags;
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::num::NonZeroI32;

mod set;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Signal(NonZeroI32);

impl Signal {
    pub fn new(raw: i32) -> Option<Self> {
        match raw {
            1..=SIG_MAXSIG => Some(Signal(unsafe { NonZeroI32::new_unchecked(raw) })),
            _ => None,
        }
    }

    pub fn get(&self) -> i32 {
        self.0.get()
    }
}

macro_rules! signals {
    ($($name:ident($num:expr),)*) => {
        $(
            #[allow(dead_code)]
            pub const $name: Signal = Signal(unsafe {
                assert!($num > 0 && $num <= SIG_MAXSIG);
                NonZeroI32::new_unchecked($num)
            });
        )*

        pub fn strsignal_impl(sig: Signal) -> Cow<'static, str> {
            match sig.0.get() {
                $( $num => Cow::Borrowed(stringify!($name)), )*
                _ => format!("{sig}", sig = sig.get()).into(),
            }
        }
    };
}

// List of PS4 signals. The value must be the same as PS4 kernel.
#[rustfmt::skip]
signals!(
    SIGHUP(1),
    SIGINT(2),
    SIGQUIT(3),
    SIGILL(4),
    SIGTRAP(5),
    SIGABRT(6),
    SIGEMT(7),
    SIGFPE(8),
    SIGKILL(9),
    SIGBUS(10),
    SIGSEGV(11),
    SIGSYS(12),
    SIGPIPE(13),
    SIGALRM(14),
    SIGTERM(15),
    SIGURG(16),
    SIGSTOP(17),
    SIGTSTP(18),
    SIGCONT(19),
    SIGCHLD(20),
    SIGTTIN(21),
    SIGTTOU(22),
    SIGIO(23),
    SIGXCPU(24),
    SIGXFSZ(25),
    SIGVTALRM(26),
    SIGPROF(27),
    SIGWINCH(28),
    SIGINFO(29),
    SIGUSR1(30),
    SIGUSR2(31),
    SIGTHR(32),
    SIGNONE(128),
);

pub fn strsignal(sig: Signal) -> Cow<'static, str> {
    // This function is generated inside the macro `signals!`.
    strsignal_impl(sig)
}

pub const SIG_MAXSIG: i32 = 128;
// List of sigprocmask operations. The value must be the same as PS4 kernel.
pub const SIG_BLOCK: i32 = 1;
pub const SIG_UNBLOCK: i32 = 2;
pub const SIG_SETMASK: i32 = 3;

pub const SIG_IGN: usize = 1;
pub const SIG_DFL: usize = 0;

/// An iterator over all possible signals
pub struct SignalIter {
    current: i32,
}

impl SignalIter {
    pub fn new() -> Self {
        Self { current: 1 }
    }
}

impl Iterator for SignalIter {
    type Item = Signal;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current <= SIG_MAXSIG {
            let signal = Signal(unsafe { NonZeroI32::new_unchecked(self.current) });
            self.current += 1;
            Some(signal)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl ExactSizeIterator for SignalIter {
    fn len(&self) -> usize {
        (SIG_MAXSIG - self.current + 1) as usize
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
        const SA_NOCLDSTOP = 0x0008;
        const SA_NODEFER = 0x0010;
        const SA_NOCLDWAIT = 0x0020;
        const SA_SIGINFO = 0x0040;
    }
}

bitflags! {
    /// Flags for SIGCHLD.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug)]
    pub struct SigChldFlags: u32 {
        const PS_NOCLDWAIT = 0x0001;
        const PS_NOCLDSTOP = 0x0002;
        const PS_CLDSIGIGN = 0x0004;
    }
}

impl Display for SignalFlags {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
