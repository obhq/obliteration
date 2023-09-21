use crate::signal::SignalSet;
use crate::ucred::{Privilege, Ucred};
use gmtx::{GroupMutex, GroupMutexWriteGuard, MutexGroup};
use llt::{SpawnError, Thread};
use std::num::NonZeroI32;
use std::sync::Arc;
use tls::{Local, Tls};

/// An implementation of `thread` structure for the main application.
///
/// See [`super::VProc`] for more information.
#[derive(Debug)]
pub struct VThread {
    id: NonZeroI32,                 // td_tid
    cred: Ucred,                    // td_ucred
    sigmask: GroupMutex<SignalSet>, // td_sigmask
}

impl VThread {
    pub(super) fn new(id: NonZeroI32, cred: Ucred, mtxg: &Arc<MutexGroup>) -> Self {
        // TODO: Check how the PS4 actually allocate the thread ID.
        Self {
            id,
            cred,
            sigmask: mtxg.new_member(SignalSet::default()),
        }
    }

    /// # Panics
    /// If the current thread does not have a [`VThread`] associated.
    pub fn current() -> Local<'static, Arc<Self>> {
        VTHREAD.get().unwrap()
    }

    pub fn id(&self) -> NonZeroI32 {
        self.id
    }

    pub fn cred(&self) -> &Ucred {
        &self.cred
    }

    pub fn sigmask_mut(&self) -> GroupMutexWriteGuard<'_, SignalSet> {
        self.sigmask.write()
    }

    /// An implementation of `priv_check`.
    pub fn has_priv(&self, p: Privilege) -> bool {
        self.cred.has_priv(p)
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

static VTHREAD: Tls<Arc<VThread>> = Tls::new();
