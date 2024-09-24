// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::vmm::hv::{CpuStates, Pstate, Sctlr, Tcr};
use std::os::fd::OwnedFd;
use thiserror::Error;

/// Implementation of [`Cpu::States`] for KVM.
pub struct KvmStates<'a> {
    cpu: &'a mut OwnedFd,
}

impl<'a> KvmStates<'a> {
    pub fn from_cpu(cpu: &'a mut OwnedFd) -> Result<Self, StatesError> {
        Ok(KvmStates { cpu })
    }
}

impl<'a> CpuStates for KvmStates<'a> {
    type Err = StatesError;

    fn set_pstate(&mut self, v: Pstate) {
        todo!()
    }

    fn set_sctlr(&mut self, v: Sctlr) {
        todo!()
    }

    fn set_mair_el1(&mut self, attrs: u64) {
        todo!()
    }

    fn set_tcr(&mut self, v: Tcr) {
        todo!()
    }

    fn set_ttbr0_el1(&mut self, baddr: usize) {
        todo!()
    }

    fn set_ttbr1_el1(&mut self, baddr: usize) {
        todo!()
    }

    fn set_sp_el1(&mut self, v: usize) {
        todo!()
    }

    fn set_pc(&mut self, v: usize) {
        todo!()
    }

    fn set_x0(&mut self, v: usize) {
        todo!()
    }

    fn set_x1(&mut self, v: usize) {
        todo!()
    }

    fn commit(self) -> Result<(), Self::Err> {
        todo!()
    }
}

/// Implementation of [`CpuStates::Err`].
#[derive(Debug, Error)]
pub enum StatesError {}
