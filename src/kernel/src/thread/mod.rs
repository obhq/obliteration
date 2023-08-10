use crate::process::VProc;
use crate::signal::SignalSet;
use llt::{SpawnError, Thread};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, RwLock, RwLockWriteGuard};
use tls::{Local, Tls};

/// An implementation of `thread` structure for the main application.
///
/// See [`crate::process::VProc`] for more information.
#[derive(Debug)]
pub struct VThread {
    id: i32,                    // td_tid
    sigmask: RwLock<SignalSet>, // td_sigmask
}

impl VThread {
    /// Spawn a new [`VThread`].
    ///
    /// The caller is responsible for `stack` deallocation.
    ///
    /// # Safety
    /// The range of memory specified by `stack` and `stack_size` must be valid throughout lifetime
    /// of the thread. Specify an unaligned stack will cause undefined behavior.
    pub unsafe fn spawn<F>(
        stack: *mut u8,
        stack_size: usize,
        mut routine: F,
    ) -> Result<Thread, SpawnError>
    where
        F: FnMut() + Send + 'static,
    {
        llt::spawn(stack, stack_size, move || {
            // This closure must not have any variables that need to be dropped on the stack. The
            // reason is because this thread will be exited without returning from the routine. That
            // mean all variables on the stack will not get dropped.
            // TODO: Check how the PS4 actually allocate the thread ID.
            let vt = Arc::new(Self {
                id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
                sigmask: RwLock::new(SignalSet::default()),
            });

            assert!(vt.id >= 0);
            assert!(VTHREAD.set(vt.clone()).is_none());

            VProc::current().push_thread(vt);

            routine();
        })
    }

    /// # Panics
    /// If the current thread does not have a [`VThread`] associated.
    pub fn current() -> Local<'static, Arc<Self>> {
        VTHREAD.get().unwrap()
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn sigmask_mut(&self) -> RwLockWriteGuard<'_, SignalSet> {
        self.sigmask.write().unwrap()
    }
}

static VTHREAD: Tls<Arc<VThread>> = Tls::new();
static NEXT_ID: AtomicI32 = AtomicI32::new(0);
