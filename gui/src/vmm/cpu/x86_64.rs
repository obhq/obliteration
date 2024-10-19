// SPDX-License-Identifier: MIT OR Apache-2.0
use super::CpuManager;
use crate::vmm::hv::Hypervisor;
use crate::vmm::screen::Screen;
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
use thiserror::Error;
use super::controller::CpuController;

pub type GdbRegs = gdbstub_arch::x86::reg::X86_64CoreRegs;

impl<H: Hypervisor, S: Screen> CpuManager<H, S> {
    fn get_cpu(&mut self, tid: Tid) -> TargetResult<&mut CpuController, Self> {
        let cpu = self
            .cpus
            .get_mut(tid.get() as usize - 1)
            .ok_or(GdbTargetError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                TargetError::CpuNotFound(tid),
            )))?;

        Ok(cpu)
    }
}

impl<H: Hypervisor, S: Screen> gdbstub::target::Target for CpuManager<H, S> {
    type Arch = X86_64_SSE;
    type Error = TargetError;

    fn base_ops(&mut self) -> BaseOps<'_, Self::Arch, Self::Error> {
        BaseOps::MultiThread(self)
    }

    fn support_breakpoints(&mut self) -> Option<BreakpointsOps<'_, Self>> {
        Some(self)
    }
}

impl<H: Hypervisor, S: Screen> MultiThreadBase for CpuManager<H, S> {
    fn read_registers(&mut self, regs: &mut X86_64CoreRegs, tid: Tid) -> TargetResult<(), Self> {
        let mut cpu = self.get_cpu(tid)?;

        let current_regs = cpu.debug_mut().unwrap().lock();

        *regs = current_regs.clone();

        Ok(())
    }

    fn write_registers(&mut self, regs: &X86_64CoreRegs, tid: Tid) -> TargetResult<(), Self> {
        let mut _cpu = self.get_cpu(tid)?;

        todo!()
    }

    fn read_addrs(
        &mut self,
        start_addr: u64,
        data: &mut [u8],
        tid: Tid,
    ) -> TargetResult<usize, Self> {
        let mut _cpu = self.get_cpu(tid)?;

        todo!()
    }

    fn write_addrs(&mut self, start_addr: u64, data: &[u8], tid: Tid) -> TargetResult<(), Self> {
        let mut _cpu = self.get_cpu(tid)?;

        todo!()
    }

    fn is_thread_alive(&mut self, tid: Tid) -> Result<bool, Self::Error> {
        todo!()
    }

    fn list_active_threads(
        &mut self,
        thread_is_active: &mut dyn FnMut(Tid),
    ) -> Result<(), Self::Error> {
        for id in (0..self.cpus.len()).map(|v| unsafe { NonZero::new_unchecked(v + 1) }) {
            thread_is_active(id);
        }

        Ok(())
    }

    #[inline(always)]
    fn support_thread_extra_info(&mut self) -> Option<ThreadExtraInfoOps<'_, Self>> {
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

impl<H: Hypervisor, S: Screen> ThreadExtraInfo for CpuManager<H, S> {
    fn thread_extra_info(&self, tid: Tid, buf: &mut [u8]) -> Result<usize, Self::Error> {
        todo!()
    }
}

/// Implementation of [`gdbstub::target::Target::Error`] for x86-64.
#[derive(Debug, Error)]
pub enum TargetError {
    #[error("cpu not found for tid: {0}")]
    CpuNotFound(Tid),
}
