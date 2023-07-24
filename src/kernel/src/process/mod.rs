use crate::errno::{Errno, EINVAL};
use crate::signal::{SignalSet, SIGKILL, SIGSTOP, SIG_BLOCK, SIG_SETMASK, SIG_UNBLOCK};
use crate::thread::VThread;
use std::collections::HashMap;
use std::num::NonZeroI32;
use std::sync::{Arc, RwLock};
use std::thread::ThreadId;
use thiserror::Error;

/// An implementation of `proc` structure represent the main application process.
///
/// Each process of the Obliteration Kernel encapsulate only one PS4 process. The reason we don't
/// encapsulate multiple PS4 processes is because there is no way to emulate `fork` with 100%
/// compatibility from the user-mode application.
pub struct VProc {
    threads: HashMap<ThreadId, Arc<RwLock<VThread>>>, // p_threads
}

impl VProc {
    pub fn new() -> Self {
        Self {
            threads: HashMap::default(),
        }
    }

    pub fn current_thread(&self) -> Arc<RwLock<VThread>> {
        let id = std::thread::current().id();
        self.threads.get(&id).unwrap().clone()
    }

    pub fn push_thread(&mut self, td: VThread) {
        let id = td.host_id();
        let td = Arc::new(RwLock::new(td));

        if self.threads.insert(id, td).is_some() {
            panic!("Thread {id:?} already have a virtual thread associated.");
        }
    }

    /// See `kern_sigprocmask` in the PS4 kernel for a reference.
    pub fn sigmask(
        &mut self,
        how: i32,
        set: Option<SignalSet>,
        oset: Option<&mut SignalSet>,
    ) -> Result<(), SigmaskError> {
        // Copy current mask to oset.
        let td = self.current_thread();
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
