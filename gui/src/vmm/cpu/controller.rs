// SPDX-License-Identifier: MIT OR Apache-2.0
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

/// Contains objects to control a CPU from outside.
pub struct CpuController {
    thread: ManuallyDrop<JoinHandle<()>>,
    debug: Arc<(Mutex<DebugStates>, Condvar)>,
    wakeup: bool,
}

impl CpuController {
    pub fn new(thread: JoinHandle<()>, debug: Arc<(Mutex<DebugStates>, Condvar)>) -> Self {
        Self {
            thread: ManuallyDrop::new(thread),
            debug,
            wakeup: false,
        }
    }

    pub fn debug_states<R>(&mut self, f: impl FnOnce(&GdbRegs) -> R) -> R {
        // Request from the CPU if not available.
        let mut s = self.debug.0.lock().unwrap();

        loop {
            s = match s.deref() {
                DebugStates::None => {
                    *s = DebugStates::Request;

                    self.wakeup = true;
                    self.debug.1.wait(s).unwrap()
                }
                DebugStates::Request => self.debug.1.wait(s).unwrap(),
                DebugStates::DebuggerOwned(v) => break f(v),
                DebugStates::CpuOwned(_) => {
                    // The CPU is not pickup the previous value yet.
                    self.debug.1.wait(s).unwrap()
                }
            };
        }
    }

    pub fn release(&mut self) {
        let mut s = self.debug.0.lock().unwrap();

        match std::mem::take(s.deref_mut()) {
            DebugStates::DebuggerOwned(v) => *s = DebugStates::CpuOwned(v),
            _ => unreachable!(),
        }

        if std::mem::take(&mut self.wakeup) {
            self.debug.1.notify_one();
        }
    }
}

impl Drop for CpuController {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::take(&mut self.thread).join().unwrap() };
    }
}

/// Debugging states of a CPU.
#[derive(Default)]
pub enum DebugStates {
    #[default]
    None,
    Request,
    DebuggerOwned(GdbRegs),
    CpuOwned(GdbRegs),
}

#[cfg(target_arch = "aarch64")]
type GdbRegs = gdbstub_arch::aarch64::reg::AArch64CoreRegs;

#[cfg(target_arch = "x86_64")]
type GdbRegs = gdbstub_arch::x86::reg::X86_64CoreRegs;
