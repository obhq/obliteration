// SPDX-License-Identifier: MIT OR Apache-2.0
use super::debug::Debuggee;
use std::mem::ManuallyDrop;
use std::thread::ScopedJoinHandle;

/// Contains objects to control a CPU from outside.
pub struct CpuController<'a> {
    thread: ManuallyDrop<ScopedJoinHandle<'a, ()>>,
    debug: ManuallyDrop<Option<Debuggee>>,
}

impl<'a> CpuController<'a> {
    pub fn new(thread: ScopedJoinHandle<'a, ()>, debug: Option<Debuggee>) -> Self {
        Self {
            thread: ManuallyDrop::new(thread),
            debug: ManuallyDrop::new(debug),
        }
    }

    pub fn debug_mut(&mut self) -> Option<&mut Debuggee> {
        self.debug.as_mut()
    }
}

impl<'a> Drop for CpuController<'a> {
    fn drop(&mut self) {
        // We need to drop the debug channel first so it will unblock the CPU thread if it is
        // waiting for a request.
        unsafe { ManuallyDrop::drop(&mut self.debug) };
        unsafe { ManuallyDrop::take(&mut self.thread).join().unwrap() };
    }
}
