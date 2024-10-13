// SPDX-License-Identifier: MIT OR Apache-2.0
use super::Debugger;
use crate::vmm::cpu::DebugStates;
use crate::vmm::hv::{Cpu, CpuExit, CpuIo};
use crate::vmm::hw::{read_u8, DeviceContext, MmioError};
use crate::vmm::VmmEvent;
use obconf::{DebuggerMemory, StopReason};
use std::error::Error;
use std::mem::offset_of;
use std::ops::Deref;
use std::ptr::null_mut;
use std::sync::{Condvar, Mutex};
use thiserror::Error;

/// Implementation of [`DeviceContext`].
pub struct Context<'a> {
    dev: &'a Debugger,
    debug: &'a (Mutex<DebugStates>, Condvar),
}

impl<'a> Context<'a> {
    pub fn new(dev: &'a Debugger, debug: &'a (Mutex<DebugStates>, Condvar)) -> Self {
        Self { dev, debug }
    }

    fn exec_stop<C: Cpu>(
        &mut self,
        exit: &mut <C::Exit<'_> as CpuExit>::Io,
        off: usize,
    ) -> Result<(), ExecError> {
        // Read stop reason.
        let stop = read_u8(exit).map_err(|e| ExecError::ReadFailed(off, e))?;
        let stop: StopReason = stop.try_into().map_err(|_| ExecError::InvalidStop(stop))?;

        self.set_states::<C>(exit, stop)?;

        // Notify GUI. This will block until the debugger works are completed.
        let stop = match stop {
            StopReason::WaitForDebugger => null_mut(),
        };

        unsafe { self.dev.event.invoke(VmmEvent::Breakpoint { stop }) };

        Ok(())
    }

    fn set_states<C: Cpu>(
        &mut self,
        exit: &mut <C::Exit<'_> as CpuExit>::Io,
        r: StopReason,
    ) -> Result<(), ExecError> {
        // Get states.
        let next = match r {
            StopReason::WaitForDebugger => Self::get_states(exit.cpu())?,
        };

        // Set states.
        let mut s = self.debug.0.lock().unwrap();

        if matches!(s.deref(), DebugStates::None) {
            *s = DebugStates::DebuggerOwned(next);
            return Ok(());
        }

        // Wait until the debugger release us.
        assert!(matches!(s.deref(), DebugStates::Request));

        *s = DebugStates::DebuggerOwned(next);

        self.debug.1.notify_one();

        loop {
            s = match s.deref() {
                DebugStates::DebuggerOwned(_) => self.debug.1.wait(s).unwrap(),
                DebugStates::CpuOwned(v) => {
                    // Two possible cases here:
                    //
                    // 1. CpuController::debug_states waiting for us to unlock.
                    // 2. CpuController::debug_states waiting for us to notify.
                    //
                    // Condvar::notify_one only wakeup the thread that already waiting.
                    *s = DebugStates::DebuggerOwned(v.clone());
                    self.debug.1.notify_one();
                    break;
                }
                _ => unreachable!(),
            };
        }

        Ok(())
    }

    #[cfg(target_arch = "aarch64")]
    fn get_states(
        _: &mut impl Cpu,
    ) -> Result<gdbstub_arch::aarch64::reg::AArch64CoreRegs, ExecError> {
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn get_states(_: &mut impl Cpu) -> Result<gdbstub_arch::x86::reg::X86_64CoreRegs, ExecError> {
        todo!()
    }
}

impl<'a, C: Cpu> DeviceContext<C> for Context<'a> {
    fn exec(&mut self, exit: &mut <C::Exit<'_> as CpuExit>::Io) -> Result<bool, Box<dyn Error>> {
        // Check field.
        let off = exit.addr() - self.dev.addr;

        if off == offset_of!(DebuggerMemory, stop) {
            self.exec_stop::<C>(exit, off)?;
        } else {
            return Err(Box::new(ExecError::UnknownField(off)));
        }

        return Ok(true);
    }
}

/// Represents an error when [`Context::exec()`] fails.
#[derive(Debug, Error)]
enum ExecError {
    #[error("unknown field at offset {0:#}")]
    UnknownField(usize),

    #[error("couldn't read data for offset {0:#}")]
    ReadFailed(usize, #[source] MmioError),

    #[error("{0:#} is not a valid stop reason")]
    InvalidStop(u8),
}
