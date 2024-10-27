pub use self::abi::*;
pub use self::pid::*;
pub use self::process::*;
pub use self::thread::*;

use crate::lock::{MappedMutex, Mutex, MutexGuard};
use alloc::sync::{Arc, Weak};
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
    pub fn new() -> Arc<Self> {
        // This function is not allowed to access the CPU context due to it can be called before the
        // context has been activated.
        Arc::new(Self {
            procs: Mutex::new(HashMap::new()),
        })
    }

    pub fn list(&self) -> MappedMutex<impl ExactSizeIterator<Item = &Weak<Proc>> + '_> {
        MutexGuard::map(self.procs.lock(), |procs| procs.values())
    }
}
