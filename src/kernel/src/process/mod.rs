pub use self::appinfo::*;
pub use self::group::*;
pub use self::rlimit::*;
pub use self::thread::*;

use crate::idt::IdTable;
use crate::rtld::Module;
use crate::ucred::{AuthInfo, Ucred};
use gmtx::{GroupMutex, GroupMutexWriteGuard, MutexGroup};
use llt::{SpawnError, Thread};
use std::num::NonZeroI32;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use thiserror::Error;

mod appinfo;
mod group;
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
    cred: Ucred,                                     // p_ucred
    group: GroupMutex<Option<VProcGroup>>,           // p_pgrp
    limits: [ResourceLimit; ResourceLimit::NLIMITS], // p_limit
    objects: GroupMutex<IdTable<ProcObj>>,
    app_info: AppInfo,
    mtxg: Arc<MutexGroup>,
}

impl VProc {
    pub fn new() -> Result<Self, VProcError> {
        // TODO: Check how ucred is constructed for a process.
        let mg = MutexGroup::new("virtual process");
        let limits = Self::load_limits()?;

        Ok(Self {
            id: Self::new_id(),
            threads: mg.new_member(Vec::new()),
            cred: Ucred::new(AuthInfo::EXE.clone()),
            group: mg.new_member(None),
            objects: mg.new_member(IdTable::new(0x1000)),
            limits,
            app_info: AppInfo::new(),
            mtxg: mg,
        })
    }

    pub fn id(&self) -> NonZeroI32 {
        self.id
    }

    pub fn cred(&self) -> &Ucred {
        &self.cred
    }

    pub fn group_mut(&self) -> GroupMutexWriteGuard<'_, Option<VProcGroup>> {
        self.group.write()
    }

    pub fn limit(&self, ty: usize) -> Option<&ResourceLimit> {
        self.limits.get(ty)
    }

    pub fn objects_mut(&self) -> GroupMutexWriteGuard<'_, IdTable<ProcObj>> {
        self.objects.write()
    }

    pub fn app_info(&self) -> &AppInfo {
        &self.app_info
    }

    pub fn mutex_group(&self) -> &Arc<MutexGroup> {
        &self.mtxg
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

        // TODO: Check how ucred is constructed for a thread.
        let cred = Ucred::new(AuthInfo::EXE.clone());
        let td = Arc::new(VThread::new(Self::new_id(), cred, &self.mtxg));
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

/// An object in the process object table.
#[derive(Debug)]
pub enum ProcObj {
    Module(Arc<Module>),
    Named(NamedObj),
}

#[derive(Debug)]
pub struct NamedObj {
    name: String,
    data: usize,
}

impl NamedObj {
    pub fn new(name: String, data: usize) -> Self {
        Self { name, data }
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
