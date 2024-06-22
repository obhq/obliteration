use self::cpu::HfCpu;
use self::ffi::hv_vcpu_create;
use self::vm::Vm;
use super::{HypervisorError, MemoryAddr, Platform, Ram};
use std::ffi::c_int;
use std::num::NonZero;
use std::sync::Arc;
use thiserror::Error;

mod cpu;
mod ffi;
mod vm;

/// Implementation of [`Platform`] using Hypervisor Framework.
///
/// Fields in this struct need to drop in a correct order.
pub struct Hf {
    vm: Vm,
    ram: Arc<Ram>,
}

impl Hf {
    pub fn new(cpu: usize, ram: Arc<Ram>) -> Result<Self, HypervisorError> {
        // Create a VM.
        let vm = Vm::new().map_err(HypervisorError::CreateVmFailed)?;

        if (vm.capability(0).map_err(HypervisorError::GetMaxCpuFailed)? as usize) < cpu {
            return Err(HypervisorError::MaxCpuTooLow);
        }

        // Map memory.
        vm.vm_map(
            ram.host_addr().cast(),
            ram.vm_addr().try_into().unwrap(),
            ram.len(),
        )
        .map_err(HypervisorError::MapRamFailed)?;

        Ok(Self { vm, ram })
    }
}

impl Platform for Hf {
    type Cpu<'a> = HfCpu<'a>;
    type CpuErr = HfCpuError;

    fn create_cpu(&self, _: usize) -> Result<Self::Cpu<'_>, Self::CpuErr> {
        let mut instance = 0;
        let ret = unsafe { hv_vcpu_create(&mut instance, 0) };

        if let Some(e) = NonZero::new(ret) {
            return Err(HfCpuError::CreateVcpuFailed(e));
        }

        Ok(HfCpu::new(instance))
    }
}

/// Implementation of [`Platform::CpuErr`].
#[derive(Debug, Error)]
pub enum HfCpuError {
    #[error("couldn't create a vCPU ({0:#x})")]
    CreateVcpuFailed(NonZero<c_int>),
}
