use self::cpu::WhpCpu;
use self::partition::Partition;
use super::{CpuFeats, Hypervisor};
use crate::vmm::ram::Ram;
use crate::vmm::VmmError;
use std::sync::Arc;
use thiserror::Error;
use windows_sys::core::HRESULT;

mod cpu;
mod partition;

pub fn new(cpu: usize, ram: Ram) -> Result<Whp, VmmError> {
    // Setup a partition.
    let mut part = Partition::new().map_err(VmmError::CreatePartitionFailed)?;

    part.set_processor_count(cpu)
        .map_err(VmmError::SetCpuCountFailed)?;
    part.setup().map_err(VmmError::SetupPartitionFailed)?;

    // Map memory.
    part.map_gpa(ram.host_addr().cast(), 0, ram.len().try_into().unwrap())
        .map_err(VmmError::MapRamFailed)?;

    Ok(Whp { part, ram })
}

/// Implementation of [`Hypervisor`] using Windows Hypervisor Platform.
///
/// Fields in this struct need to drop in a correct order.
pub struct Whp {
    part: Partition,
    ram: Ram,
}

impl Hypervisor for Whp {
    type Cpu<'a> = WhpCpu<'a>;
    type CpuErr = WhpCpuError;

    fn cpu_features(&mut self) -> Result<CpuFeats, Self::CpuErr> {
        Ok(CpuFeats {})
    }

    fn ram(&self) -> &Ram {
        &self.ram
    }

    fn ram_mut(&mut self) -> &mut Ram {
        &mut self.ram
    }

    fn create_cpu(&self, id: usize) -> Result<Self::Cpu<'_>, Self::CpuErr> {
        let id = id.try_into().unwrap();

        self.part
            .create_virtual_processor(id)
            .map_err(WhpCpuError::CreateVirtualProcessorFailed)
    }
}

/// Implementation of [`Hypervisor::CpuErr`].
#[derive(Debug, Error)]
pub enum WhpCpuError {
    #[error("couldn't create a virtual processor ({0:#x})")]
    CreateVirtualProcessorFailed(HRESULT),
}
