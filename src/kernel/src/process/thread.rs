use super::{CpuMask, CpuSet, VProc, NEXT_ID};
use crate::errno::Errno;
use crate::fs::VFile;
use crate::signal::SignalSet;
use crate::ucred::{Privilege, PrivilegeError, Ucred};
use bitflags::bitflags;
use gmtx::{Gutex, GutexGroup, GutexReadGuard, GutexWriteGuard};
use llt::{OsThread, SpawnError};
use std::num::NonZeroI32;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use thiserror::Error;
use tls::{Local, Tls};

/// An implementation of `thread` structure for the main application.
///
/// See [`super::VProc`] for more information.
#[derive(Debug)]
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
    pub fn new(proc: Arc<VProc>, cred: &Arc<Ucred>) -> Self {
        // TODO: Check how the PS4 actually allocate the thread ID.
        let gg = GutexGroup::new();

        Self {
            proc,
            id: NonZeroI32::new(NEXT_ID.fetch_add(1, Ordering::Relaxed)).unwrap(),
            cred: cred.clone(),
            sigmask: gg.spawn(SignalSet::default()),
            pri_class: 3, // TODO: Check the actual value on the PS4 when a thread is created.
            base_user_pri: 700, // TODO: Same here.
            pcb: gg.spawn(Pcb {
                fsbase: 0,
                flags: PcbFlags::empty(),
            }),
            cpuset: CpuSet::new(CpuMask::default()), // TODO: Same here.
            name: gg.spawn(None),                    // TODO: Same here
            fpop: gg.spawn(None),
        }
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

    /// Start the thread.
    ///
    /// The caller is responsible for `stack` deallocation.
    ///
    /// # Safety
    /// The range of memory specified by `stack` and `stack_size` must be valid throughout lifetime
    /// of the thread. Specify an unaligned stack will cause undefined behavior.
    pub unsafe fn start<F>(
        self,
        stack: *mut u8,
        stack_size: usize,
        mut routine: F,
    ) -> Result<OsThread, SpawnError>
    where
        F: FnMut() + Send + 'static,
    {
        let proc = self.proc.clone();
        let td = Arc::new(self);
        let running = Running(td.clone());
        // Lock the list before spawn the thread to prevent race condition if the new thread run
        // too fast and found out they is not in our list.
        let mut threads = proc.threads.write();
        let raw = llt::spawn(stack, stack_size, move || {
            // This closure must not have any variables that need to be dropped on the stack. The
            // reason is because this thread will be exited without returning from the routine. That
            // mean all variables on the stack will not get dropped.
            assert!(VTHREAD.set(running.0.clone()).is_none());
            routine();
        })?;

        // Add to the list.
        threads.push(td);

        Ok(raw)
    }
}

/// An implementation of `pcb` structure.
#[derive(Debug)]
pub struct Pcb {
    fsbase: usize,   // pcb_fsbase
    flags: PcbFlags, // pcb_flags
}

impl Pcb {
    pub fn fsbase(&self) -> usize {
        self.fsbase
    }

    pub fn set_fsbase(&mut self, v: usize) {
        self.fsbase = v;
    }

    pub fn flags_mut(&mut self) -> &mut PcbFlags {
        &mut self.flags
    }
}

bitflags! {
    /// Flags of [`Pcb`].
    #[derive(Debug)]
    pub struct PcbFlags: u32 {
        const PCB_FULL_IRET = 0x01;
    }
}

// An object for removing the thread from the list when dropped.
struct Running(Arc<VThread>);

impl Drop for Running {
    fn drop(&mut self) {
        let mut threads = self.0.proc.threads.write();
        let index = threads.iter().position(|td| td.id == self.0.id).unwrap();

        threads.remove(index);
    }
}

static VTHREAD: Tls<Arc<VThread>> = Tls::new();

#[derive(Debug, Error)]
pub enum FileAllocError {}

impl Errno for FileAllocError {
    fn errno(&self) -> NonZeroI32 {
        match *self {}
    }
}
