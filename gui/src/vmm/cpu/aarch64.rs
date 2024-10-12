// SPDX-License-Identifier: MIT OR Apache-2.0
use super::CpuManager;
use crate::vmm::hv::Hypervisor;
use crate::vmm::screen::Screen;
use gdbstub::target::ext::base::BaseOps;
use thiserror::Error;

impl<H: Hypervisor, S: Screen> gdbstub::target::Target for CpuManager<H, S> {
    type Arch = Arch;
    type Error = TargetError;

    fn base_ops(&mut self) -> BaseOps<'_, Self::Arch, Self::Error> {
        todo!()
    }
}

/// Implementation of [`gdbstub::arch::Arch`] for AArch64.
pub enum Arch {}

impl gdbstub::arch::Arch for Arch {
    type Usize = u64;
    type Registers = Registers;
    type BreakpointKind = BreakpointKind;
    type RegId = RegId;
}

/// Implementation of [`gdbstub::arch::Registers`] for AArch64.
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

/// Implementation of [`gdbstub::arch::BreakpointKind`] for AArch64.
#[derive(Debug)]
pub struct BreakpointKind {}

impl gdbstub::arch::BreakpointKind for BreakpointKind {
    fn from_usize(kind: usize) -> Option<Self> {
        todo!()
    }
}

/// Implementation of [`gdbstub::arch::RegId`] for AArch64.
#[derive(Debug)]
pub struct RegId {}

impl gdbstub::arch::RegId for RegId {
    fn from_raw_id(id: usize) -> Option<(Self, Option<std::num::NonZeroUsize>)> {
        todo!()
    }
}

/// Implementation of [`gdbstub::target::Target::Error`] for AArch64.
#[derive(Debug, Error)]
pub enum TargetError {}
