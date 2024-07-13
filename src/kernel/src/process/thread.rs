use super::{CpuMask, CpuSet, Pcb, ProcEvents, VProc};
use crate::errno::Errno;
use crate::event::EventSet;
use crate::fs::VFile;
use crate::signal::SignalSet;
use crate::ucred::{CanSeeError, Privilege, PrivilegeError, Ucred};
use gmtx::{Gutex, GutexGroup, GutexReadGuard, GutexWriteGuard};
use llt::{OsThread, SpawnError};
use macros::Errno;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;
use tls::{Local, Tls};

/// An implementation of `thread` structure.
pub struct VThread {
    proc: Arc<VProc>,            // td_proc
    id: NonZeroI32,              // td_tid
    cred: Arc<Ucred>,            // td_ucred
    sigmask: Gutex<SignalSet>,   // td_sigmask
    pri_class: u16,              // td_pri_class
    base_user_pri: u16,          // td_base_user_pri
    pcb: Gutex<Pcb>,             // td_pcb
    cpuset: CpuSet,              // td_cpuset
    name: Gutex<Option<String>>, // td_name
    fpop: Gutex<Option<VFile>>,  // td_fpop
}

impl VThread {
    pub(super) fn new(
        proc: &Arc<VProc>,
        id: NonZeroI32,
        events: &Arc<EventSet<ProcEvents>>,
    ) -> Arc<Self> {
        let gg = GutexGroup::new();
        let cred = proc.cred().clone();
        let mut td = Self {
            proc: proc.clone(),
            id,
            cred,
            sigmask: gg.spawn(SignalSet::default()),
            pri_class: 3, // TODO: Check the actual value on the PS4 when a thread is created.
            base_user_pri: 700, // TODO: Same here.
            pcb: gg.spawn(Pcb::default()),
            cpuset: CpuSet::new(CpuMask::default()), // TODO: Same here.
            name: gg.spawn(None),                    // TODO: Same here
            fpop: gg.spawn(None),
        };

        // Trigger thread_init event.
        let mut et = events.trigger();

        for h in et.select(|s| &s.thread_init) {
            h(&mut td);
        }

        // Trigger thread_ctor event.
        let td = Arc::new(td);
        let weak = Arc::downgrade(&td);

        for h in et.select(|s| &s.thread_ctor) {
            h(&weak);
        }

        drop(et);

        td
    }

    /// Return [`None`] if the calling thread is not a PS4 thread.
    pub fn current() -> Option<Local<'static, Arc<Self>>> {
        VTHREAD.get()
    }

    pub fn proc(&self) -> &Arc<VProc> {
        &self.proc
    }

    pub fn id(&self) -> NonZeroI32 {
        self.id
    }

    pub fn cred(&self) -> &Arc<Ucred> {
        &self.cred
    }

    pub fn sigmask_mut(&self) -> GutexWriteGuard<'_, SignalSet> {
        self.sigmask.write()
    }

    pub fn pri_class(&self) -> u16 {
        self.pri_class
    }

    pub fn base_user_pri(&self) -> u16 {
        self.base_user_pri
    }

    pub fn pcb(&self) -> GutexReadGuard<'_, Pcb> {
        self.pcb.read()
    }

    pub fn pcb_mut(&self) -> GutexWriteGuard<'_, Pcb> {
        self.pcb.write()
    }

    pub fn cpuset(&self) -> &CpuSet {
        &self.cpuset
    }

    pub fn set_name(&self, name: Option<&str>) {
        *self.name.write() = name.map(|n| n.to_owned());
    }

    pub fn set_fpop(&self, file: Option<VFile>) {
        *self.fpop.write() = file
    }

    /// An implementation of `priv_check`.
    pub fn priv_check(&self, p: Privilege) -> Result<(), PrivilegeError> {
        self.cred.priv_check(p)
    }

    /// Determine whether current thread may reschedule `p`.
    ///
    /// See `p_cansched` on the PS4 for a reference.
    pub fn can_sched(&self, p: &Arc<VProc>) -> Result<(), CanSchedError> {
        if Arc::ptr_eq(&self.proc, p) {
            return Ok(());
        }

        todo!()
    }

    /// Determine if current thread "can see" the subject specified by `p`.
    ///
    /// See `p_cansee` on the PS4 for a reference.
    pub fn can_see(&self, p: &Arc<VProc>) -> Result<(), CanSeeError> {
        self.cred.can_see(p.cred())
    }

    /// Start the thread.
    ///
    /// The caller is responsible for `stack` deallocation.
    ///
    /// # Safety
    /// The range of memory specified by `stack` and `stack_size` must be valid throughout lifetime
    /// of the thread. Specify an unaligned stack will cause undefined behavior.
    pub unsafe fn start<F>(
        self: &Arc<Self>,
        stack: *mut u8,
        stack_size: usize,
        mut routine: F,
    ) -> Result<OsThread, SpawnError>
    where
        F: FnMut() + Send + 'static,
    {
        let running = Running(self.clone());
        let raw = llt::spawn(stack, stack_size, move || {
            // This closure must not have any variables that need to be dropped on the stack. The
            // reason is because this thread will be exited without returning from the routine. That
            // mean all variables on the stack will not get dropped.
            assert!(VTHREAD.set(running.0.clone()).is_none());
            routine();
        })?;

        Ok(raw)
    }
}

// An object for removing the thread from the list when dropped.
struct Running(Arc<VThread>);

impl Drop for Running {
    fn drop(&mut self) {
        let mut threads = self.0.proc.threads_mut();
        let index = threads.iter().position(|td| td.id == self.0.id).unwrap();

        threads.remove(index);
    }
}

/// Represents an error when [`VThread::can_sched()`] fails.
#[derive(Debug, Error, Errno)]
pub enum CanSchedError {}

static VTHREAD: Tls<Arc<VThread>> = Tls::new();
