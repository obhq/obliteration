use self::cpu::WhpCpu;
use self::partition::Partition;
use super::{HypervisorError, MemoryAddr, Platform, Ram};
use std::sync::Arc;
use thiserror::Error;
use windows_sys::core::HRESULT;

mod cpu;
mod partition;

/// Implementation of [`Platform`] using Windows Hypervisor Platform.
///
/// Fields in this struct need to drop in a correct order.
pub struct Whp {
    part: Partition,
    ram: Arc<Ram>,
}

impl Whp {
    pub fn new(cpu: usize, ram: Arc<Ram>) -> Result<Self, HypervisorError> {
        // Setup a partition.
        let mut part = Partition::new().map_err(HypervisorError::CreatePartitionFailed)?;

        part.set_processor_count(cpu)
            .map_err(HypervisorError::SetCpuCountFailed)?;
        part.setup()
            .map_err(HypervisorError::SetupPartitionFailed)?;

        // Map memory.
        part.map_gpa(
            ram.host_addr().cast(),
            ram.vm_addr().try_into().unwrap(),
            ram.len().try_into().unwrap(),
        )
        .map_err(HypervisorError::MapRamFailed)?;

        Ok(Self { part, ram })
    }
}

impl Platform for Whp {
    type Cpu<'a> = WhpCpu<'a>;
    type CpuErr = WhpCpuError;

    fn create_cpu(&self, id: usize) -> Result<Self::Cpu<'_>, Self::CpuErr> {
        let id = id.try_into().unwrap();

        self.part
            .create_virtual_processor(id)
            .map_err(WhpCpuError::CreateVirtualProcessorFailed)
    }
}

/// Implementation of [`Platform::CpuErr`].
#[derive(Debug, Error)]
pub enum WhpCpuError {
    #[error("couldn't create a virtual processor ({0:#x})")]
    CreateVirtualProcessorFailed(HRESULT),
}
