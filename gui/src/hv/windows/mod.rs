// SPDX-License-Identifier: MIT OR Apache-2.0
use self::cpu::WhpCpu;
use self::mapper::WhpMapper;
use self::partition::Partition;
use super::{CpuFeats, Hypervisor, Ram};
use std::num::NonZero;
use thiserror::Error;
use windows_sys::core::HRESULT;

mod cpu;
mod mapper;
mod partition;

/// Panics
/// If `ram_size` is not multiply by `ram_block`.
///
/// # Safety
/// `ram_block` must be greater or equal host page size.
pub unsafe fn new(
    cpu: usize,
    ram_size: NonZero<usize>,
    ram_block: NonZero<usize>,
    debug: bool,
) -> Result<impl Hypervisor, WhpError> {
    // Create RAM.
    let ram = Ram::new(ram_size, ram_block, WhpMapper).map_err(WhpError::CreateRamFailed)?;

    // Setup a partition.
    let mut part = Partition::new().map_err(WhpError::CreatePartitionFailed)?;

    part.set_processor_count(cpu)
        .map_err(WhpError::SetCpuCountFailed)?;
    part.setup().map_err(WhpError::SetupPartitionFailed)?;

    // Map memory.
    part.map_gpa(
        ram.host_addr().cast(),
        0,
        ram.len().get().try_into().unwrap(),
    )
    .map_err(WhpError::MapRamFailed)?;

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
    ram: Ram<WhpMapper>,
}

impl Hypervisor for Whp {
    type Mapper = WhpMapper;
    type Cpu<'a> = WhpCpu<'a>;
    type CpuErr = WhpCpuError;

    fn cpu_features(&self) -> &CpuFeats {
        &self.feats
    }

    fn ram(&self) -> &Ram<Self::Mapper> {
        &self.ram
    }

    fn ram_mut(&mut self) -> &mut Ram<Self::Mapper> {
        &mut self.ram
    }

    fn create_cpu(&self, id: usize) -> Result<Self::Cpu<'_>, Self::CpuErr> {
        let id = id.try_into().unwrap();

        self.part
            .create_virtual_processor(id)
            .map_err(WhpCpuError::CreateVirtualProcessorFailed)
    }
}

/// Represents an error when [`Whp`] fails to initialize.
#[derive(Debug, Error)]
pub enum WhpError {
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
