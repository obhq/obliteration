use super::cpu::HfCpu;
use crate::vmm::hv::{CpuExit, CpuIo, IoBuf};
use std::error::Error;

/// Implementation of [`CpuExit`] for Hypervisor Framework.
pub struct HfExit<'a, 'b>(&'a mut HfCpu<'b>);

impl<'a, 'b> HfExit<'a, 'b> {
    pub fn new(cpu: &'a mut HfCpu<'b>) -> Self {
        Self(cpu)
    }
}

impl<'a, 'b> CpuExit for HfExit<'a, 'b> {
    type Io = HfIo;

    fn into_io(self) -> Result<Self::Io, Self> {
        todo!();
    }
}

/// Implementation of [`CpuIo`] for Hypervisor Framework.
pub struct HfIo {}

impl CpuIo for HfIo {
    fn addr(&self) -> usize {
        todo!();
    }

    fn buffer(&mut self) -> IoBuf {
        todo!();
    }

    fn translate(&self, vaddr: usize) -> Result<usize, Box<dyn Error>> {
        todo!();
    }
}
