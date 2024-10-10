use super::cpu::HfCpu;
use crate::vmm::hv::{CpuExit, CpuIo, IoBuf};
use std::error::Error;
use std::marker::PhantomData;

/// Implementation of [`CpuExit`] for Hypervisor Framework.
pub struct HfExit<'a, 'b> {
    cpu: PhantomData<&'a mut HfCpu<'b>>,
    exit_reason: u64,
}

impl<'a, 'b> HfExit<'a, 'b> {
    pub fn new(exit_reason: u64) -> Self {
        Self {
            cpu: PhantomData,
            exit_reason,
        }
    }
}

impl<'a, 'b> CpuExit for HfExit<'a, 'b> {
    type Io = HfIo;

    fn into_hlt(self) -> Result<(), Self> {
        match self.exit_reason.try_into() {
            Ok(hv_sys::VMX_REASON_HLT) => Ok(()),
            _ => Err(self),
        }
    }

    fn into_io(self) -> Result<Self::Io, Self> {
        todo!();
    }
}

/// Implementation of [`CpuIo`] for Hypervisor Framework.
pub struct HfIo {}

impl CpuIo for HfIo {
    type TranslateErr = std::io::Error;

    fn addr(&self) -> usize {
        todo!();
    }

    fn buffer(&mut self) -> IoBuf {
        todo!();
    }

    fn translate(&self, vaddr: usize) -> Result<usize, std::io::Error> {
        todo!();
    }
}
