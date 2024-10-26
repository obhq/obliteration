// SPDX-License-Identifier: MIT OR Apache-2.0
use super::{CpuManager, GdbError};
use crate::screen::Screen;
use crate::vmm::hv::Hypervisor;
use gdbstub::target::ext::base::BaseOps;
use gdbstub::target::ext::breakpoints::{
    Breakpoints, BreakpointsOps, SwBreakpoint, SwBreakpointOps,
};
use gdbstub::target::TargetResult;

pub type GdbRegs = gdbstub_arch::aarch64::reg::AArch64CoreRegs;

pub(super) const BREAKPOINT_SIZE: NonZero<usize> = unsafe { NonZero::new_unchecked(4) };

impl<H: Hypervisor, S: Screen> gdbstub::target::Target for CpuManager<H, S> {
    type Arch = gdbstub_arch::aarch64::AArch64;
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
        todo!()
    }

    fn remove_sw_breakpoint(&mut self, addr: u64, kind: usize) -> TargetResult<bool, Self> {
        todo!()
    }
}
