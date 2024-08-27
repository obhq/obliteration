use self::cpu::HfCpu;
use self::vm::Vm;
use super::Hypervisor;
use crate::vmm::hw::Ram;
use crate::vmm::VmmError;
use hv_sys::hv_vcpu_create;
use std::ffi::c_int;
use std::num::NonZero;
use std::sync::Arc;
use thiserror::Error;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod cpu;
mod vm;

/// Implementation of [`Hypervisor`] using Hypervisor Framework.
///
/// Fields in this struct need to drop in a correct order.
pub struct Hf {
    vm: Vm,
    ram: Arc<Ram>,
}

impl Hf {
    pub fn new(_: usize, ram: Arc<Ram>) -> Result<Self, VmmError> {
        // Create a VM.
        let vm = Vm::new().map_err(VmmError::CreateVmFailed)?;

        // Map memory.
        vm.vm_map(ram.host_addr().cast_mut().cast(), 0, ram.len())
            .map_err(VmmError::MapRamFailed)?;

        Ok(Self { vm, ram })
    }
}

impl Hypervisor for Hf {
    type Cpu<'a> = HfCpu<'a>;
    type CpuErr = HfCpuError;

    fn create_cpu(&self, _: usize) -> Result<Self::Cpu<'_>, Self::CpuErr> {
        let mut instance = 0;

        #[cfg(target_arch = "x86_64")]
        {
            let ret = unsafe { hv_vcpu_create(&mut instance, 0) };

            if let Some(e) = NonZero::new(ret) {
                return Err(HfCpuError::CreateVcpuFailed(e));
            }

            Ok(HfCpu::new_x64(instance))
        }

        #[cfg(target_arch = "aarch64")]
        {
            let mut exit = std::ptr::null_mut();

            let ret = unsafe { hv_vcpu_create(&mut instance, &mut exit, std::ptr::null_mut()) };

            if let Some(e) = NonZero::new(ret) {
                return Err(HfCpuError::CreateVcpuFailed(e));
            }

            Ok(HfCpu::new_aarch64(instance, exit))
        }
    }
}

/// Implementation of [`Hypervisor::CpuErr`].
#[derive(Debug, Error)]
pub enum HfCpuError {
    #[error("couldn't create a vCPU ({0:#x})")]
    CreateVcpuFailed(NonZero<c_int>),
}
