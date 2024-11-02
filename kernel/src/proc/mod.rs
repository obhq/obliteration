pub use self::abi::*;
pub use self::pid::*;
pub use self::process::*;
pub use self::thread::*;

use crate::event::{Event, EventSet};
use crate::lock::{MappedMutex, Mutex, MutexGuard};
use crate::signal::Signal;
use crate::subsystem::Subsystem;
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
    events: Arc<EventSet<ProcEvents>>,
}

impl ProcMgr {
    /// # Context safety
    /// This function does not require a CPU context.
    pub fn new() -> Arc<Self> {
        let events = Arc::default();

        Arc::new(Self {
            procs: Mutex::new(HashMap::new()),
            events,
        })
    }

    pub fn list(&self) -> MappedMutex<impl ExactSizeIterator<Item = &Weak<Proc>> + '_> {
        MutexGuard::map(self.procs.lock(), |procs| procs.values())
    }

    /// We imply `RFSTOPPED` to make [`ProcMgr`] not depend on a scheduler.
    ///
    /// See `fork1` on the PS4 for a reference.
    pub fn fork(&self, abi: Arc<dyn ProcAbi>, flags: Fork) -> Result<Arc<Proc>, ForkError> {
        // TODO: Refactor this for readability.
        if (flags.into_bits() & 0x60008f8b) != 0
            || flags.copy_fd() && flags.clear_fd()
            || flags.parent_signal().into_bits() != 0 && !flags.custom_signal()
        {
            return Err(ForkError::InvalidFlags);
        }

        if !flags.create_process() {
            todo!()
        }

        // Create process.
        Ok(Proc::new(abi, &self.events))
    }
}

impl Subsystem for ProcMgr {}

/// Events that related to a process.
#[derive(Default)]
pub struct ProcEvents {
    pub process_init: Event<fn(&mut Proc)>,
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
    /// Create a child process.
    ///
    /// This has the same value as `RFPROC`.
    pub create_process: bool,
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
