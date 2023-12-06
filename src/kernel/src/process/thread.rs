use super::{CpuMask, CpuSet};
use crate::signal::SignalSet;
use crate::ucred::{Privilege, PrivilegeError, Ucred};
use bitflags::bitflags;
use gmtx::{Gutex, GutexGroup, GutexReadGuard, GutexWriteGuard};
use llt::{SpawnError, Thread};
use std::num::NonZeroI32;
use std::sync::Arc;
use tls::{Local, Tls};

/// An implementation of `thread` structure for the main application.
///
/// See [`super::VProc`] for more information.
#[derive(Debug)]
pub struct VThread {
    id: NonZeroI32,            // td_tid
    cred: Ucred,               // td_ucred
    sigmask: Gutex<SignalSet>, // td_sigmask
    pri_class: u16,            // td_pri_class
    base_user_pri: u16,        // td_base_user_pri
    pcb: Gutex<Pcb>,           // td_pcb
    cpuset: CpuSet,            // td_cpuset
    name: Gutex<[u8; 32]>,     // td_name
}

impl VThread {
    pub(super) fn new(id: NonZeroI32, cred: Ucred, gg: &Arc<GutexGroup>) -> Self {
        // TODO: Check how the PS4 actually allocate the thread ID.
        Self {
            id,
            cred,
            sigmask: gg.spawn(SignalSet::default()),
            pri_class: 3, // TODO: Check the actual value on the PS4 when a thread is created.
            base_user_pri: 120, // TODO: Same here.
            pcb: gg.spawn(Pcb {
                fsbase: 0,
                flags: PcbFlags::empty(),
            }),
            cpuset: CpuSet::new(CpuMask::default()), // TODO: Same here.
            name: gg.spawn([0u8; 32]),               // TODO: Same here
        }
    }

    /// Return [`None`] if the calling thread is not a PS4 thread.
    pub fn current() -> Option<Local<'static, Arc<Self>>> {
        VTHREAD.get()
    }

    pub fn id(&self) -> NonZeroI32 {
        self.id
    }

    pub fn cred(&self) -> &Ucred {
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

    pub fn set_name(&self, name: &str) {
        self.name.write().copy_from_slice(name.as_bytes());
    }

    /// An implementation of `priv_check`.
    pub fn priv_check(&self, p: Privilege) -> Result<(), PrivilegeError> {
        self.cred.priv_check(p)
    }

    pub(super) unsafe fn spawn<F>(
        self: &Arc<Self>,
        stack: *mut u8,
        stack_size: usize,
        mut routine: F,
    ) -> Result<Thread, SpawnError>
    where
        F: FnMut() + Send + 'static,
    {
        let mut td = Some(self.clone());

        llt::spawn(stack, stack_size, move || {
            // This closure must not have any variables that need to be dropped on the stack. The
            // reason is because this thread will be exited without returning from the routine. That
            // mean all variables on the stack will not get dropped.
            assert!(VTHREAD.set(td.take().unwrap()).is_none());
            routine();
        })
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

static VTHREAD: Tls<Arc<VThread>> = Tls::new();
