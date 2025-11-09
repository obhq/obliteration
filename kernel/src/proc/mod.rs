pub use self::abi::*;
pub use self::pid::*;
pub use self::process::*;
pub use self::thread::*;

use crate::event::{Event, EventSet};
use crate::lock::{MappedMutex, Mutex, MutexGuard};
use crate::signal::Signal;
use crate::subsystem::Subsystem;
use alloc::sync::{Arc, Weak};
use core::error::Error;
use core::fmt::{Display, Formatter};
use hashbrown::HashMap;
use macros::bitflag;
use rustc_hash::FxBuildHasher;

mod abi;
mod cell;
mod pid;
mod process;
mod thread;

/// Manage all processes in the system.
pub struct ProcMgr {
    procs: Mutex<HashMap<Pid, Weak<Proc>, FxBuildHasher>>, // allproc + pidhashtbl + zombproc
    events: Arc<EventSet<ProcEvents>>,
}

impl ProcMgr {
    pub fn new() -> Arc<Self> {
        let events = Arc::default();

        Arc::new(Self {
            procs: Mutex::new(HashMap::with_hasher(FxBuildHasher)),
            events,
        })
    }

    pub fn list(&self) -> MappedMutex<'_, impl ExactSizeIterator<Item = &Weak<Proc>>> {
        MutexGuard::map(self.procs.lock(), |procs| procs.values())
    }

    /// We imply `RFSTOPPED` to make [`ProcMgr`] not depend on the scheduler.
    ///
    /// See `fork1` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x14B830|
    pub fn fork(&self, abi: Arc<dyn ProcAbi>, flags: Fork) -> Result<Arc<Proc>, ForkError> {
        // TODO: Refactor this for readability.
        if (u32::from(flags) & 0x60008f8b) != 0
            || flags.has_all(Fork::CopyFd | Fork::ClearFd)
            || flags.has_any(Fork::ParentSignal.mask()) && !flags.has_any(Fork::CustomSignal)
        {
            return Err(ForkError::InvalidFlags);
        }

        if !flags.has_any(Fork::CreateProcess) {
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
    pub process_ctor: Event<fn(&Weak<Proc>)>,
}

/// Flags to control behavior of [`ProcMgr::fork()`].
#[bitflag(u32)]
pub enum Fork {
    /// Duplicate file descriptor table to the child instead of sharing it with the parent. Cannot
    /// used together with [`Self::clear_fd()`].
    ///
    /// This has the same value as `RFFDG`.
    CopyFd = 0x4,
    /// Create a child process.
    ///
    /// This has the same value as `RFPROC`.
    CreateProcess = 0x10,
    /// Create an empty file descriptor table for the child. Cannot used together with
    /// [`Self::copy_fd()`].
    ///
    /// This has the same value as `RFCFDG`.
    ClearFd = 0x1000,
    /// Enable [`Self::parent_signal()`].
    ///
    /// This has the same value as `RFTSIGZMB`.
    CustomSignal = 0x80000,
    /// Use this signal instead of `SIGCHLD` to notify the parent. Requires
    /// [`Self::custom_signal()`] to be enabled.
    ///
    /// This has the same value produced by `RFTSIGNUM` macro.
    ParentSignal(Signal) = 0xFF00000,
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
