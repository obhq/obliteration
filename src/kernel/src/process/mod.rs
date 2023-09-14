pub use self::appinfo::*;
pub use self::rlimit::*;
pub use self::thread::*;

use crate::ucred::Ucred;
use gmtx::{GroupMutex, MutexGroup};
use llt::{SpawnError, Thread};
use std::num::NonZeroI32;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use thiserror::Error;

mod appinfo;
mod rlimit;
mod thread;

/// An implementation of `proc` structure represent the main application process.
///
/// Each process of the Obliteration Kernel encapsulate only one PS4 process. The reason we don't
/// encapsulate multiple PS4 processes is because there is no way to emulate `fork` with 100%
/// compatibility from the user-mode application.
#[derive(Debug)]
pub struct VProc {
    id: NonZeroI32,                                  // p_pid
    threads: GroupMutex<Vec<Arc<VThread>>>,          // p_threads
    limits: [ResourceLimit; ResourceLimit::NLIMITS], // p_limit
    app_info: AppInfo,
    mtxg: Arc<MutexGroup>,
}

impl VProc {
    pub fn new() -> Result<Self, VProcError> {
        let mtxg = MutexGroup::new();
        let limits = Self::load_limits()?;

        Ok(Self {
            id: Self::new_id(),
            threads: mtxg.new_member(Vec::new()),
            limits,
            app_info: AppInfo::new(),
            mtxg,
        })
    }

    pub fn id(&self) -> NonZeroI32 {
        self.id
    }

    pub fn limit(&self, ty: usize) -> Option<&ResourceLimit> {
        self.limits.get(ty)
    }

    pub fn app_info(&self) -> &AppInfo {
        &self.app_info
    }

    /// Spawn a new [`VThread`].
    ///
    /// The caller is responsible for `stack` deallocation.
    ///
    /// # Safety
    /// The range of memory specified by `stack` and `stack_size` must be valid throughout lifetime
    /// of the thread. Specify an unaligned stack will cause undefined behavior.
    pub unsafe fn new_thread<F>(
        &'static self,
        stack: *mut u8,
        stack_size: usize,
        mut routine: F,
    ) -> Result<Thread, SpawnError>
    where
        F: FnMut() + Send + 'static,
    {
        // Lock the list before spawn the thread to prevent race condition if the new thread run
        // too fast and found out they is not in our list.
        let mut threads = self.threads.write();
        let td = Arc::new(VThread::new(Self::new_id(), Ucred::new(), &self.mtxg));
        let active = Box::new(ActiveThread {
            proc: self,
            id: td.id(),
        });

        // Spawn the thread.
        let host = td.spawn(stack, stack_size, move || {
            // We cannot have any variables that need to be dropped before invoke the routine.
            assert_eq!(VThread::current().id(), active.id); // We want to drop active when exited.
            routine();
        })?;

        // Add to the list.
        threads.push(td);

        Ok(host)
    }

    fn load_limits() -> Result<[ResourceLimit; ResourceLimit::NLIMITS], VProcError> {
        type R = ResourceLimit;
        type E = VProcError;

        Ok([
            R::new(R::CPU).map_err(E::GetCpuLimitFailed)?,
            R::new(R::FSIZE).map_err(E::GetFileSizeLimitFailed)?,
            R::new(R::DATA).map_err(E::GetDataLimitFailed)?,
        ])
    }

    fn new_id() -> NonZeroI32 {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

        // Just in case if the user manage to spawn 2,147,483,647 threads in a single run so we
        // don't encountered a weird bug.
        assert!(id > 0);

        NonZeroI32::new(id).unwrap()
    }
}

// An object for removing the thread from the list when dropped.
struct ActiveThread {
    proc: &'static VProc,
    id: NonZeroI32,
}

impl Drop for ActiveThread {
    fn drop(&mut self) {
        let mut threads = self.proc.threads.write();
        let index = threads.iter().position(|td| td.id() == self.id).unwrap();

        threads.remove(index);
    }
}

/// Represents an error when [`VProc`] construction is failed.
#[derive(Debug, Error)]
pub enum VProcError {
    #[error("cannot get CPU time limit")]
    GetCpuLimitFailed(#[source] std::io::Error),

    #[error("cannot get file size limit")]
    GetFileSizeLimitFailed(#[source] std::io::Error),

    #[error("cannot get data size limit")]
    GetDataLimitFailed(#[source] std::io::Error),
}

static NEXT_ID: AtomicI32 = AtomicI32::new(1);
