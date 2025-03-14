// SPDX-License-Identifier: MIT OR Apache-2.0
use self::cpu::WhpCpu;
use self::partition::Partition;
use super::{CpuFeats, Hypervisor, Ram};
use std::num::NonZero;
use thiserror::Error;
use windows_sys::core::HRESULT;

mod cpu;
mod partition;

/// `page_size` is a page size on the VM. This value will be used as a block size if it is larger
/// than page size on the host otherwise block size will be page size on the host.
///
/// `ram_size` must be multiply by the block size calculated from the above.
pub fn new(
    cpu: usize,
    ram_size: NonZero<usize>,
    page_size: NonZero<usize>,
    debug: bool,
) -> Result<impl Hypervisor, HvError> {
    // Create RAM.
    let ram = Ram::new(page_size, ram_size, ())?;

    // Setup a partition.
    let mut part = Partition::new().map_err(HvError::CreatePartitionFailed)?;

    part.set_processor_count(cpu)
        .map_err(HvError::SetCpuCountFailed)?;
    part.setup().map_err(HvError::SetupPartitionFailed)?;

    // Map memory.
    part.map_gpa(
        ram.host_addr().cast(),
        0,
        ram.len().get().try_into().unwrap(),
    )
    .map_err(HvError::MapRamFailed)?;

    Ok(Whp {
        part,
        feats: CpuFeats {},
        ram,
    })
}

/// Implementation of [`Hypervisor`] using Windows Hypervisor Platform.
///
/// Fields in this struct need to drop in a correct order.
struct Whp {
    part: Partition,
    feats: CpuFeats,
    ram: Ram,
}

impl Hypervisor for Whp {
    type Cpu<'a> = WhpCpu<'a>;
    type CpuErr = WhpCpuError;

    fn cpu_features(&self) -> &CpuFeats {
        &self.feats
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

/// Represents an error when operation on WHP fails.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum HvError {
    #[error("couldn't get host page size")]
    GetHostPageSize(#[source] std::io::Error),

    #[error("size of RAM is not valid")]
    InvalidRamSize,

    #[error("couldn't create a RAM")]
    CreateRamFailed(#[source] std::io::Error),

    #[error("couldn't create WHP partition object ({0:#x})")]
    CreatePartitionFailed(HRESULT),

    #[error("couldn't set number of CPU ({0:#x})")]
    SetCpuCountFailed(HRESULT),

    #[error("couldn't setup WHP partition ({0:#x})")]
    SetupPartitionFailed(HRESULT),

    #[error("couldn't map the RAM to WHP partition ({0:#x})")]
    MapRamFailed(HRESULT),
}

/// Implementation of [`Hypervisor::CpuErr`].
#[derive(Debug, Error)]
pub enum WhpCpuError {
    #[error("couldn't create a virtual processor ({0:#x})")]
    CreateVirtualProcessorFailed(HRESULT),
}
