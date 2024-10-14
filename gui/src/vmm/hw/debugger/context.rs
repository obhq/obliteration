// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::vmm::cpu::GdbRegs;
use crate::vmm::debug::Debugger;
use crate::vmm::hv::{Cpu, CpuExit, CpuIo};
use crate::vmm::hw::{read_u8, DeviceContext, MmioError};
use crate::vmm::VmmEvent;
use obconf::{DebuggerMemory, StopReason};
use std::error::Error;
use std::mem::offset_of;
use std::ptr::null_mut;
use thiserror::Error;

/// Implementation of [`DeviceContext`].
pub struct Context<'a> {
    dev: &'a super::Debugger,
    debug: Debugger<GdbRegs>,
}

impl<'a> Context<'a> {
    pub fn new(dev: &'a super::Debugger, debug: Debugger<GdbRegs>) -> Self {
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
        let regs = match stop {
            StopReason::WaitForDebugger => Self::get_regs(exit.cpu())?,
        };

        // Notify GUI. This will block until the debugger has completed their works.
        let resp = self.debug.send(regs);
        let stop = match stop {
            StopReason::WaitForDebugger => null_mut(),
        };

        unsafe { self.dev.event.invoke(VmmEvent::Breakpoint { stop }) };

        // Update registers from debugger.
        Self::set_regs(exit.cpu(), resp.into_response())
    }

    #[cfg(target_arch = "aarch64")]
    fn get_regs(_: &mut impl Cpu) -> Result<GdbRegs, ExecError> {
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn get_regs(_: &mut impl Cpu) -> Result<GdbRegs, ExecError> {
        todo!()
    }

    #[cfg(target_arch = "aarch64")]
    fn set_regs(_: &mut impl Cpu, _: GdbRegs) -> Result<(), ExecError> {
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn set_regs(_: &mut impl Cpu, _: GdbRegs) -> Result<(), ExecError> {
        todo!()
    }
}

impl<'a, C: Cpu> DeviceContext<C> for Context<'a> {
    fn mmio(&mut self, exit: &mut <C::Exit<'_> as CpuExit>::Io) -> Result<bool, Box<dyn Error>> {
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
