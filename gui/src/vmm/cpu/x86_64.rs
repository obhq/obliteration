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
use gdbstub::target::TargetResult;
use thiserror::Error;

impl<H: Hypervisor, S: Screen> gdbstub::target::Target for CpuManager<H, S> {
    type Arch = Arch;
    type Error = TargetError;

    fn base_ops(&mut self) -> BaseOps<'_, Self::Arch, Self::Error> {
        BaseOps::MultiThread(self)
    }

    fn support_breakpoints(&mut self) -> Option<BreakpointsOps<'_, Self>> {
        Some(self)
    }
}

impl<H: Hypervisor, S: Screen> MultiThreadBase for CpuManager<H, S> {
    fn read_registers(&mut self, regs: &mut Registers, tid: Tid) -> TargetResult<(), Self> {
        todo!()
    }

    fn write_registers(&mut self, regs: &Registers, tid: Tid) -> TargetResult<(), Self> {
        todo!()
    }

    fn read_addrs(
        &mut self,
        start_addr: u64,
        data: &mut [u8],
        tid: Tid,
    ) -> TargetResult<usize, Self> {
        todo!()
    }

    fn write_addrs(&mut self, start_addr: u64, data: &[u8], tid: Tid) -> TargetResult<(), Self> {
        todo!()
    }

    fn list_active_threads(
        &mut self,
        thread_is_active: &mut dyn FnMut(Tid),
    ) -> Result<(), Self::Error> {
        todo!()
    }
}

impl<H: Hypervisor, S: Screen> Breakpoints for CpuManager<H, S> {
    fn support_sw_breakpoint(&mut self) -> Option<SwBreakpointOps<'_, Self>> {
        Some(self)
    }
}

impl<H: Hypervisor, S: Screen> SwBreakpoint for CpuManager<H, S> {
    fn add_sw_breakpoint(&mut self, addr: u64, kind: BreakpointKind) -> TargetResult<bool, Self> {
        todo!()
    }

    fn remove_sw_breakpoint(
        &mut self,
        addr: u64,
        kind: BreakpointKind,
    ) -> TargetResult<bool, Self> {
        todo!()
    }
}

/// Implementation of [`gdbstub::arch::Arch`] for x86-64.
pub enum Arch {}

impl gdbstub::arch::Arch for Arch {
    type Usize = u64;
    type Registers = Registers;
    type BreakpointKind = BreakpointKind;
    type RegId = RegId;
}

/// Implementation of [`gdbstub::arch::Registers`] for x86-64.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Registers {}

impl gdbstub::arch::Registers for Registers {
    type ProgramCounter = u64;

    fn pc(&self) -> Self::ProgramCounter {
        todo!()
    }

    fn gdb_serialize(&self, write_byte: impl FnMut(Option<u8>)) {
        todo!()
    }

    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
        todo!()
    }
}

/// Implementation of [`gdbstub::arch::BreakpointKind`] for x86-64.
#[derive(Debug)]
pub struct BreakpointKind {}

impl gdbstub::arch::BreakpointKind for BreakpointKind {
    fn from_usize(kind: usize) -> Option<Self> {
        todo!()
    }
}

/// Implementation of [`gdbstub::arch::RegId`] for x86-64.
#[derive(Debug)]
pub struct RegId {}

impl gdbstub::arch::RegId for RegId {
    fn from_raw_id(id: usize) -> Option<(Self, Option<std::num::NonZeroUsize>)> {
        todo!()
    }
}

/// Implementation of [`gdbstub::target::Target::Error`] for x86-64.
#[derive(Debug, Error)]
pub enum TargetError {}
