use crate::errno::{Errno, EINVAL};
use crate::signal::{SignalSet, SIGKILL, SIGSTOP, SIG_BLOCK, SIG_SETMASK, SIG_UNBLOCK};
use crate::thread::VThread;
use std::num::NonZeroI32;
use std::sync::{Arc, OnceLock, RwLock};
use thiserror::Error;

/// An implementation of `proc` structure represent the main application process.
///
/// Each process of the Obliteration Kernel encapsulate only one PS4 process. The reason we don't
/// encapsulate multiple PS4 processes is because there is no way to emulate `fork` with 100%
/// compatibility from the user-mode application.
#[derive(Debug)]
pub struct VProc {
    threads: Vec<Arc<RwLock<VThread>>>, // p_threads
}

impl VProc {
    /// # Panics
    /// If this method has been called a second time.
    pub fn new() -> &'static RwLock<Self> {
        let vp = RwLock::new(Self {
            threads: Vec::new(),
        });

        VPROC.set(vp).unwrap();

        unsafe { VPROC.get().unwrap_unchecked() }
    }

    /// # Panics
    /// If [`new()`] has not been called before calling this method.
    pub fn current() -> &'static RwLock<Self> {
        VPROC.get().unwrap()
    }

    pub fn push_thread(&mut self, vt: Arc<RwLock<VThread>>) {
        self.threads.push(vt);
    }

    pub fn remove_thread(&mut self, id: i32) -> Option<Arc<RwLock<VThread>>> {
        let index = self
            .threads
            .iter()
            .position(|vt| vt.read().unwrap().id() == id)?;

        Some(self.threads.remove(index))
    }

    /// See `kern_sigprocmask` in the PS4 kernel for a reference.
    pub fn sigmask(
        &mut self,
        how: i32,
        set: Option<SignalSet>,
        oset: Option<&mut SignalSet>,
    ) -> Result<(), SigmaskError> {
        // Copy current mask to oset. We need to do this inside the mutable VProc because we want
        // to maintain the same behavior on the PS4, which lock the process when doing sigprocmask.
        let td = VThread::current();
        let mut td = td.write().unwrap();

        if let Some(v) = oset {
            *v = *td.sigmask();
        }

        // Update the mask.
        let mut set = match set {
            Some(v) => v,
            None => return Ok(()),
        };

        match how {
            SIG_BLOCK => {
                // Remove uncatchable signals.
                set.remove(SIGKILL);
                set.remove(SIGSTOP);

                // Update mask.
                *td.sigmask_mut() |= set;
            }
            SIG_UNBLOCK => {
                // Update mask.
                *td.sigmask_mut() &= !set;

                // TODO: Invoke signotify at the end.
            }
            SIG_SETMASK => {
                // Remove uncatchable signals.
                set.remove(SIGKILL);
                set.remove(SIGSTOP);

                // Replace mask.
                *td.sigmask_mut() = set;

                // TODO: Invoke signotify at the end.
            }
            v => return Err(SigmaskError::InvalidHow(v)),
        }

        // TODO: Check if we need to invoke reschedule_signals.
        Ok(())
    }
}

/// Represents an error when [`VProc::sigmask()`] is failed.
#[derive(Debug, Error)]
pub enum SigmaskError {
    #[error("{0} is not a valid how")]
    InvalidHow(i32),
}

impl Errno for SigmaskError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::InvalidHow(_) => EINVAL,
        }
    }
}

static VPROC: OnceLock<RwLock<VProc>> = OnceLock::new();
