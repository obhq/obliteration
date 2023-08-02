use crate::signal::SignalSet;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
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
    /// Create a new [`VThread`] for the calling thread.
    ///
    /// # Panics
    /// If the current thread already have a [`VThread`] associated.
    pub fn new() -> Arc<Self> {
        // TODO: Check how the PS4 actually allocate the thread ID.
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

        assert!(id >= 0);

        // Associate the VThread to the calling thread.
        let vt = Arc::new(Self {
            id,
            sigmask: RwLock::new(SignalSet::default()),
        });

        if VTHREAD.set(vt.clone()).is_some() {
            panic!("The calling thread already has a virtual thread associated.");
        }

        vt
    }

    /// # Panics
    /// If the current thread does not have a [`VThread`] associated.
    pub fn current() -> Local<'static, Arc<Self>> {
        VTHREAD.get().unwrap()
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn sigmask(&self) -> RwLockReadGuard<'_, SignalSet> {
        self.sigmask.read().unwrap()
    }

    pub fn sigmask_mut(&self) -> RwLockWriteGuard<'_, SignalSet> {
        self.sigmask.write().unwrap()
    }
}

static VTHREAD: Tls<Arc<VThread>> = Tls::new();
static NEXT_ID: AtomicI32 = AtomicI32::new(0);
