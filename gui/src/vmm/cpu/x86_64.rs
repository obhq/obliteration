// SPDX-License-Identifier: MIT OR Apache-2.0
use super::controller::CpuController;
use super::{CpuManager, GdbError};
use crate::screen::Screen;
use crate::vmm::hv::Hypervisor;
use gdbstub::common::Tid;
use gdbstub::target::ext::base::multithread::MultiThreadBase;
use gdbstub::target::ext::base::BaseOps;
use gdbstub::target::ext::breakpoints::{
    Breakpoints, BreakpointsOps, SwBreakpoint, SwBreakpointOps,
};
use gdbstub::target::ext::thread_extra_info::{ThreadExtraInfo, ThreadExtraInfoOps};
use gdbstub::target::{TargetError as GdbTargetError, TargetResult};
use gdbstub_arch::x86::reg::X86_64CoreRegs;
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
        todo!()
    }

    fn remove_sw_breakpoint(&mut self, addr: u64, kind: usize) -> TargetResult<bool, Self> {
        todo!()
    }
}
