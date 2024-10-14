// SPDX-License-Identifier: MIT OR Apache-2.0
use super::CpuManager;
use crate::vmm::hv::Hypervisor;
use crate::vmm::screen::Screen;
use gdbstub::target::ext::base::BaseOps;
use thiserror::Error;

pub type GdbRegs = gdbstub_arch::aarch64::reg::AArch64CoreRegs;

impl<H: Hypervisor, S: Screen> gdbstub::target::Target for CpuManager<H, S> {
    type Arch = gdbstub_arch::aarch64::AArch64;
    type Error = TargetError;

    fn base_ops(&mut self) -> BaseOps<'_, Self::Arch, Self::Error> {
        todo!()
    }
}

/// Implementation of [`gdbstub::target::Target::Error`] for AArch64.
#[derive(Debug, Error)]
pub enum TargetError {}
