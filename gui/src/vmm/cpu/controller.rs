// SPDX-License-Identifier: MIT OR Apache-2.0
use super::GdbRegs;
use crate::vmm::debug::Debuggee;
use std::mem::ManuallyDrop;
use std::thread::JoinHandle;

/// Contains objects to control a CPU from outside.
pub struct CpuController {
    thread: ManuallyDrop<JoinHandle<()>>,
    debug: Debuggee<GdbRegs>,
}

impl CpuController {
    pub fn new(thread: JoinHandle<()>, debug: Debuggee<GdbRegs>) -> Self {
        Self {
            thread: ManuallyDrop::new(thread),
            debug,
        }
    }

    pub fn debug_mut(&mut self) -> &mut Debuggee<GdbRegs> {
        &mut self.debug
    }
}

impl Drop for CpuController {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::take(&mut self.thread).join().unwrap() };
    }
}
