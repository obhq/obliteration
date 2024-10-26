// SPDX-License-Identifier: MIT OR Apache-2.0
use super::{CpuManager, GdbError};
use crate::screen::Screen;
use crate::vmm::hv::Hypervisor;
use gdbstub::target::ext::base::BaseOps;
use gdbstub::target::ext::breakpoints::{
    Breakpoints, BreakpointsOps, SwBreakpoint, SwBreakpointOps,
};
use gdbstub::target::{TargetError, TargetResult};
use gdbstub_arch::x86::X86_64_SSE;
use std::num::NonZero;

pub type GdbRegs = gdbstub_arch::x86::reg::X86_64CoreRegs;

impl<H: Hypervisor, S: Screen> gdbstub::target::Target for CpuManager<H, S> {
    type Arch = X86_64_SSE;
    type Error = GdbError;

    fn base_ops(&mut self) -> BaseOps<'_, Self::Arch, Self::Error> {
        BaseOps::MultiThread(self)
    }

    fn support_breakpoints(&mut self) -> Option<BreakpointsOps<'_, Self>> {
        Some(self)
    }
}

impl<H: Hypervisor, S: Screen> Breakpoints for CpuManager<H, S> {
    fn support_sw_breakpoint(&mut self) -> Option<SwBreakpointOps<'_, Self>> {
        Some(self)
    }
}

impl<H: Hypervisor, S: Screen> SwBreakpoint for CpuManager<H, S> {
    fn add_sw_breakpoint(&mut self, addr: u64, kind: usize) -> TargetResult<bool, Self> {
        if self.sw_breakpoints.contains_key(&addr) {
            return Ok(false);
        }

        let breakpoint_size = NonZero::new(kind).unwrap();

        let cpu = self
            .cpus
            .get_mut(0)
            .ok_or(TargetError::Errno(Self::GDB_ENOENT))?;

        let translated_addr = cpu
            .debug_mut()
            .unwrap()
            .translate_address(addr.try_into().unwrap())
            .ok_or(TargetError::Errno(Self::GDB_ENOENT))?;

        // Get data.
        let src = self
            .hv
            .ram()
            .lock(translated_addr, breakpoint_size)
            .ok_or(TargetError::Errno(Self::GDB_EFAULT))?
            .as_mut_ptr();

        let code_slice = unsafe { std::slice::from_raw_parts_mut(src, breakpoint_size.get()) };

        let mut bytes = Vec::new();

        bytes.extend_from_slice(code_slice);

        // INT3
        code_slice.fill(0xCC);

        self.sw_breakpoints.insert(addr, bytes.into_boxed_slice());

        Ok(true)
    }

    fn remove_sw_breakpoint(&mut self, addr: u64, kind: usize) -> TargetResult<bool, Self> {
        let Some(breakpoint) = self.sw_breakpoints.remove(&addr) else {
            return Ok(false);
        };

        let breakpoint_size = NonZero::new(kind).unwrap();

        if breakpoint.len() != breakpoint_size.get() {
            todo!();
        }

        let cpu = self
            .cpus
            .get_mut(0)
            .ok_or(TargetError::Errno(Self::GDB_ENOENT))?;

        let translated_addr = cpu
            .debug_mut()
            .unwrap()
            .translate_address(addr.try_into().unwrap())
            .ok_or(TargetError::Errno(Self::GDB_ENOENT))?;

        // Get data.
        let src = self
            .hv
            .ram()
            .lock(translated_addr, breakpoint_size)
            .ok_or(TargetError::Errno(Self::GDB_EFAULT))?
            .as_mut_ptr();

        let code_slice = unsafe { std::slice::from_raw_parts_mut(src, breakpoint_size.get()) };

        code_slice.copy_from_slice(&breakpoint);

        Ok(true)
    }
}
