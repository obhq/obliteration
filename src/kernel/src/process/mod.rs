use crate::syscalls::Syscalls;
use std::sync::atomic::AtomicI32;
use std::sync::Arc;

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
}

static NEXT_ID: AtomicI32 = AtomicI32::new(123);
