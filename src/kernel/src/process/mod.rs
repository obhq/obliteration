use crate::thread::VThread;
use std::sync::{Arc, OnceLock, RwLock};

/// An implementation of `proc` structure represent the main application process.
///
/// Each process of the Obliteration Kernel encapsulate only one PS4 process. The reason we don't
/// encapsulate multiple PS4 processes is because there is no way to emulate `fork` with 100%
/// compatibility from the user-mode application.
#[derive(Debug)]
pub struct VProc {
    threads: RwLock<Vec<Arc<VThread>>>, // p_threads
}

impl VProc {
    /// # Panics
    /// If this method has been called a second time.
    pub fn new() -> &'static Self {
        let vp = Self {
            threads: RwLock::new(Vec::new()),
        };

        VPROC.set(vp).unwrap();

        unsafe { VPROC.get().unwrap_unchecked() }
    }

    /// # Panics
    /// If [`new()`] has not been called before calling this method.
    pub fn current() -> &'static Self {
        VPROC.get().unwrap()
    }

    pub fn push_thread(&self, vt: Arc<VThread>) {
        self.threads.write().unwrap().push(vt);
    }

    pub fn remove_thread(&self, id: i32) -> Option<Arc<VThread>> {
        let mut threads = self.threads.write().unwrap();
        let index = threads.iter().position(|vt| vt.id() == id)?;

        Some(threads.remove(index))
    }
}

static VPROC: OnceLock<VProc> = OnceLock::new();
