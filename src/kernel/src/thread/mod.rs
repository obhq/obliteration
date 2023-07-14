use crate::signal::SignalSet;
use std::thread::ThreadId;

/// An implementation of `thread` structure for the main application.
///
/// See [`crate::process::VProc`] for more information.
pub struct VThread {
    host_id: ThreadId,
    sigmask: SignalSet, // td_sigmask
}

impl VThread {
    pub fn new(host_id: ThreadId) -> Self {
        Self {
            host_id,
            sigmask: SignalSet::default(),
        }
    }

    pub fn host_id(&self) -> ThreadId {
        self.host_id
    }

    pub fn sigmask(&self) -> &SignalSet {
        &self.sigmask
    }

    pub fn sigmask_mut(&mut self) -> &mut SignalSet {
        &mut self.sigmask
    }
}
