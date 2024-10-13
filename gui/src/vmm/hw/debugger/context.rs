// SPDX-License-Identifier: MIT OR Apache-2.0
use super::Debugger;
use crate::vmm::cpu::CpuState;
use crate::vmm::hv::{Cpu, CpuExit, CpuIo};
use crate::vmm::hw::{read_u8, DeviceContext, MmioError};
use crate::vmm::VmmEvent;
use obconf::{DebuggerMemory, StopReason};
use std::error::Error;
use std::mem::offset_of;
use std::ptr::null_mut;
use std::sync::Mutex;
use thiserror::Error;

/// Implementation of [`DeviceContext`].
pub struct Context<'a> {
    dev: &'a Debugger,
    state: &'a Mutex<CpuState>,
}

impl<'a> Context<'a> {
    pub fn new(dev: &'a Debugger, state: &'a Mutex<CpuState>) -> Self {
        Self { dev, state }
    }
}

impl<'a, C: Cpu> DeviceContext<C> for Context<'a> {
    fn exec(&mut self, exit: &mut <C::Exit<'_> as CpuExit>::Io) -> Result<bool, Box<dyn Error>> {
        // Check field.
        let off = exit.addr() - self.dev.addr;

        if off == offset_of!(DebuggerMemory, stop) {
            // Read stop reason.
            let stop = read_u8(exit).map_err(|e| ExecError::ReadFailed(off, e))?;
            let stop: StopReason = stop
                .try_into()
                .map_err(|_| Box::new(ExecError::InvalidStop(stop)))?;

            // Notify GUI.
            let stop = match stop {
                StopReason::WaitForDebugger => null_mut(),
            };

            unsafe { self.dev.event.invoke(VmmEvent::Breakpoint { stop }) };
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
