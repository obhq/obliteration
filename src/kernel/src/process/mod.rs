use crate::budget::ProcType;
use crate::dev::DmemContainer;
use crate::fs::Vnode;
use crate::syscalls::Syscalls;
use crate::ucred::AuthInfo;
use crate::vm::MemoryManagerError;
use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use thiserror::Error;

pub use self::appinfo::*;
pub use self::binary::*;
pub use self::cpuset::*;
pub use self::filedesc::*;
pub use self::group::*;
pub use self::proc::*;
pub use self::rlimit::*;
pub use self::session::*;
pub use self::signal::*;
pub use self::thread::*;

mod appinfo;
mod binary;
mod cpuset;
mod filedesc;
mod group;
mod proc;
mod rlimit;
mod session;
mod signal;
mod thread;

/// Manage all PS4 processes.
pub struct ProcManager {}

impl ProcManager {
    pub fn new(sys: &mut Syscalls) -> Arc<Self> {
        let pmgr = Arc::new(Self {});

        pmgr
    }

    /// See `fork1` on the PS4 for a reference.
    pub fn spawn(
        &self,
        auth: AuthInfo,
        budget_id: usize,
        budget_ptype: ProcType,
        dmem_container: DmemContainer,
        root: Arc<Vnode>,
        system_path: impl Into<String>,
        mut sys: Syscalls,
    ) -> Result<Arc<VProc>, SpawnError> {
        VProc::new(
            auth,
            budget_id,
            budget_ptype,
            dmem_container,
            root,
            system_path,
            sys,
        )
    }
}

/// Represents an error when [`ProcManager::spawn()`] was failed.
#[derive(Debug, Error)]
pub enum SpawnError {
    #[error("failed to load limits")]
    FailedToLoadLimits(#[from] LoadLimitError),

    #[error("virtual memory initialization failed")]
    VmInitFailed(#[from] MemoryManagerError),
}

static NEXT_ID: AtomicI32 = AtomicI32::new(123);
