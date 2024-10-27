// SPDX-License-Identifier: MIT OR Apache-2.0
use super::debug::Debuggee;
use std::mem::ManuallyDrop;
use std::thread::JoinHandle;

/// Contains objects to control a CPU from outside.
pub struct CpuController {
    thread: ManuallyDrop<JoinHandle<()>>,
    debug: ManuallyDrop<Option<Debuggee>>,
    pub resume_action: Option<ResumeAction>,
}

impl CpuController {
    pub fn new(thread: JoinHandle<()>, debug: Option<Debuggee>) -> Self {
        Self {
            thread: ManuallyDrop::new(thread),
            debug: ManuallyDrop::new(debug),
            resume_action: None,
        }
    }

    pub fn debug_mut(&mut self) -> Option<&mut Debuggee> {
        self.debug.as_mut()
    }
}

impl Drop for CpuController {
    fn drop(&mut self) {
        // We need to drop the debug channel first so it will unblock the CPU thread if it is
        // waiting for a request.
        unsafe { ManuallyDrop::drop(&mut self.debug) };
        unsafe { ManuallyDrop::take(&mut self.thread).join().unwrap() };
    }
}

pub(super) enum ResumeAction {
    Continue,
}
