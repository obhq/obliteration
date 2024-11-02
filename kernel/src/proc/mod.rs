pub use self::abi::*;
pub use self::pid::*;
pub use self::process::*;
pub use self::thread::*;

use crate::lock::{MappedMutex, Mutex, MutexGuard};
use crate::signal::Signal;
use alloc::sync::{Arc, Weak};
use bitfield_struct::bitfield;
use core::error::Error;
use core::fmt::{Display, Formatter};
use hashbrown::HashMap;

mod abi;
mod pid;
mod process;
mod thread;

/// Manage all processes in the system.
pub struct ProcMgr {
    procs: Mutex<HashMap<Pid, Weak<Proc>>>, // allproc + pidhashtbl + zombproc
}

impl ProcMgr {
    /// # Context safety
    /// This function does not require a CPU context.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            procs: Mutex::new(HashMap::new()),
        })
    }

    pub fn list(&self) -> MappedMutex<impl ExactSizeIterator<Item = &Weak<Proc>> + '_> {
        MutexGuard::map(self.procs.lock(), |procs| procs.values())
    }

    /// Our implementation imply `RFPROC` since it is always specified on the PS4.
    ///
    /// We also imply `RFSTOPPED` to make [`ProcMgr`] not depend on a scheduler.
    ///
    /// See `fork1` on the PS4 for a reference.
    pub fn fork(&self, flags: Fork) -> Result<Arc<Proc>, ForkError> {
        // TODO: Refactor this for readability.
        if (flags.into_bits() & 0x60008f8b) != 0
            || flags.copy_fd() && flags.clear_fd()
            || flags.parent_signal().into_bits() != 0 && !flags.custom_signal()
        {
            return Err(ForkError::InvalidFlags);
        }

        todo!()
    }
}

/// Flags to control behavior of [`ProcMgr::fork()`].
#[bitfield(u32)]
pub struct Fork {
    __: bool,
    __: bool,
    /// Duplicate file descriptor table to the child instead of sharing it with the parent. Cannot
    /// used together with [`Self::clear_fd()`].
    ///
    /// This has the same value as `RFFDG`.
    pub copy_fd: bool,
    __: bool,
    __: bool,
    __: bool,
    __: bool,
    __: bool,
    __: bool,
    __: bool,
    __: bool,
    __: bool,
    /// Create an empty file descriptor table for the child. Cannot used together with
    /// [`Self::copy_fd()`].
    ///
    /// This has the same value as `RFCFDG`.
    pub clear_fd: bool,
    __: bool,
    __: bool,
    __: bool,
    __: bool,
    __: bool,
    __: bool,
    /// Enable [`Self::parent_signal()`].
    ///
    /// This has the same value as `RFTSIGZMB`.
    pub custom_signal: bool,
    /// Use this signal instead of `SIGCHLD` to notify the parent. Requires
    /// [`Self::custom_signal()`] to be enabled.
    ///
    /// This has the same value produced by `RFTSIGNUM` macro.
    #[bits(8)]
    pub parent_signal: Signal,
    __: bool,
    __: bool,
    __: bool,
    __: bool,
}

/// Represents an error when [`ProcMgr::fork()`] fails.
#[derive(Debug)]
pub enum ForkError {
    InvalidFlags,
}

impl Error for ForkError {}

impl Display for ForkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidFlags => f.write_str("invalid flags"),
        }
    }
}
