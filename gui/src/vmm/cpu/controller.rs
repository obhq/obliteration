// SPDX-License-Identifier: MIT OR Apache-2.0
use std::mem::ManuallyDrop;
use std::thread::JoinHandle;

/// Contains objects to control a CPU from outside.
pub struct CpuController {
    thread: ManuallyDrop<JoinHandle<()>>,
}

impl CpuController {
    pub fn new(thread: JoinHandle<()>) -> Self {
        Self {
            thread: ManuallyDrop::new(thread),
        }
    }
}

impl Drop for CpuController {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::take(&mut self.thread).join().unwrap() };
    }
}
