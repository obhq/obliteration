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
use std::collections::hash_map::Entry;
use std::num::NonZero;

pub type GdbRegs = gdbstub_arch::x86::reg::X86_64CoreRegs;

pub(super) const BREAKPOINT_SIZE: NonZero<usize> = unsafe { NonZero::new_unchecked(1) };
const BREAKPOINT_BYTES: [u8; BREAKPOINT_SIZE.get()] = [0xCC];

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
    fn add_sw_breakpoint(&mut self, addr: u64, _kind: usize) -> TargetResult<bool, Self> {
        let Entry::Vacant(entry) = self.sw_breakpoints.entry(addr) else {
            return Ok(false);
        };

        let cpu = self.cpus.first_mut().unwrap();

        let translated_addr = cpu
            .debug_mut()
            .unwrap()
            .translate_address(addr.try_into().unwrap())
            .ok_or(TargetError::Fatal(GdbError::MainCpuExited))?;

        // Get data.
        let mut src = self
            .hv
            .ram()
            .lock(translated_addr, BREAKPOINT_SIZE)
            .ok_or(TargetError::Errno(Self::GDB_EFAULT))?;

        let code_slice =
            unsafe { std::slice::from_raw_parts_mut(src.as_mut_ptr(), BREAKPOINT_SIZE.get()) };

        let mut code_bytes = std::mem::replace(code_slice, BREAKPOINT_BYTES);

        entry.insert(code_bytes);

        Ok(true)
    }

    fn remove_sw_breakpoint(&mut self, addr: u64, _kind: usize) -> TargetResult<bool, Self> {
        let Some(breakpoint) = self.sw_breakpoints.remove(&addr) else {
            return Ok(false);
        };

        let cpu = self.cpus.first_mut().unwrap();

        let translated_addr = cpu
            .debug_mut()
            .unwrap()
            .translate_address(addr.try_into().unwrap())
            .ok_or(TargetError::Fatal(GdbError::MainCpuExited))?;

        // Get data.
        let mut src = self
            .hv
            .ram()
            .lock(translated_addr, BREAKPOINT_SIZE)
            .ok_or(TargetError::Errno(Self::GDB_EFAULT))?;

        let code_slice =
            unsafe { std::slice::from_raw_parts_mut(src.as_mut_ptr(), BREAKPOINT_SIZE.get()) };

        code_slice.copy_from_slice(&breakpoint);

        Ok(true)
    }
}
