// SPDX-License-Identifier: MIT OR Apache-2.0
use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// Contains objects to control a CPU from outside.
pub struct CpuController {
    thread: ManuallyDrop<JoinHandle<()>>,
    state: Arc<Mutex<CpuState>>,
}

impl CpuController {
    pub fn new(thread: JoinHandle<()>, state: Arc<Mutex<CpuState>>) -> Self {
        Self {
            thread: ManuallyDrop::new(thread),
            state,
        }
    }
}

impl Drop for CpuController {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::take(&mut self.thread).join().unwrap() };
    }
}

/// State of a CPU.
pub enum CpuState {
    Running,
}
