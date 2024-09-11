// SPDX-License-Identifier: MIT OR Apache-2.0
use self::cpu::HfCpu;
use super::{CpuFeats, Hypervisor};
use crate::vmm::ram::Ram;
use crate::vmm::VmmError;
use hv_sys::{hv_return_t, hv_vcpu_create, hv_vm_create, hv_vm_destroy, hv_vm_map};
use std::num::NonZero;
use thiserror::Error;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod cpu;

pub fn new(_: usize, ram: Ram) -> Result<impl Hypervisor, VmmError> {
    Hvf::new(ram)
}

/// Implementation of [`Hypervisor`] using Hypervisor Framework.
struct Hvf {
    ram: Ram,
    #[cfg(target_arch = "aarch64")]
    cpu_config: hv_sys::hv_vcpu_config_t,
}

impl Hvf {
    fn new(ram: Ram) -> Result<Self, VmmError> {
        // Create a VM.
        #[cfg(target_arch = "aarch64")]
        let ret = unsafe { hv_vm_create(std::ptr::null_mut()) };
        #[cfg(target_arch = "x86_64")]
        let ret = unsafe { hv_vm_create(0) };
        let hv = match NonZero::new(ret) {
            Some(ret) => return Err(VmmError::CreateVmFailed(ret)),
            None => Self {
                ram,
                #[cfg(target_arch = "aarch64")]
                cpu_config: unsafe { hv_sys::hv_vcpu_config_create() },
            },
        };

        // Set RAM.
        let host = hv.ram.host_addr().cast_mut().cast();
        let len = hv.ram.len().try_into().unwrap();
        let ret = unsafe { hv_vm_map(host, 0, len, 1 | 2 | 4) };

        match NonZero::new(ret) {
            Some(ret) => Err(VmmError::MapRamFailed(ret)),
            None => Ok(hv),
        }
    }

    #[cfg(target_arch = "aarch64")]
    fn read_feature_reg(
        &mut self,
        reg: hv_sys::hv_feature_reg_t,
    ) -> Result<u64, NonZero<hv_return_t>> {
        use hv_sys::hv_vcpu_config_get_feature_reg;

        let mut val = 0;
        let ret = unsafe { hv_vcpu_config_get_feature_reg(self.cpu_config, reg, &mut val) };

        match NonZero::new(ret) {
            Some(e) => Err(e),
            None => Ok(val),
        }
    }
}

impl Drop for Hvf {
    fn drop(&mut self) {
        // Free CPU config.
        #[cfg(target_arch = "aarch64")]
        unsafe {
            os_release(self.cpu_config.cast())
        };

        // Destroy VM.
        let status = unsafe { hv_vm_destroy() };

        if status != 0 {
            panic!("hv_vm_destroy() fails with {status:#x}");
        }
    }
}

impl Hypervisor for Hvf {
    type Cpu<'a> = HfCpu<'a>;
    type CpuErr = HvfCpuError;

    #[cfg(target_arch = "aarch64")]
    fn cpu_features(&mut self) -> Result<CpuFeats, Self::CpuErr> {
        use hv_sys::{
            hv_feature_reg_t_HV_FEATURE_REG_ID_AA64MMFR0_EL1 as HV_FEATURE_REG_ID_AA64MMFR0_EL1,
            hv_feature_reg_t_HV_FEATURE_REG_ID_AA64MMFR1_EL1 as HV_FEATURE_REG_ID_AA64MMFR1_EL1,
        };

        let mmfr0 = self
            .read_feature_reg(HV_FEATURE_REG_ID_AA64MMFR0_EL1)
            .map_err(HvfCpuError::ReadMmfr0Failed)?;
        let mmfr1 = self
            .read_feature_reg(HV_FEATURE_REG_ID_AA64MMFR1_EL1)
            .map_err(HvfCpuError::ReadMmfr1Failed)?;

        Ok(CpuFeats {
            mmfr0: mmfr0.into(),
            mmfr1: mmfr1.into(),
        })
    }

    #[cfg(target_arch = "x86_64")]
    fn cpu_features(&mut self) -> Result<CpuFeats, Self::CpuErr> {
        Ok(CpuFeats {})
    }

    fn ram(&self) -> &Ram {
        &self.ram
    }

    fn ram_mut(&mut self) -> &mut Ram {
        &mut self.ram
    }

    #[cfg(target_arch = "aarch64")]
    fn create_cpu(&self, _: usize) -> Result<Self::Cpu<'_>, Self::CpuErr> {
        let mut instance = 0;
        let mut exit = std::ptr::null_mut();
        let ret = unsafe { hv_vcpu_create(&mut instance, &mut exit, self.cpu_config) };

        match NonZero::new(ret) {
            Some(e) => Err(HvfCpuError::CreateVcpuFailed(e)),
            None => Ok(HfCpu::new_aarch64(instance, exit)),
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn create_cpu(&self, _: usize) -> Result<Self::Cpu<'_>, Self::CpuErr> {
        let mut instance = 0;
        let ret = unsafe { hv_vcpu_create(&mut instance, 0) };

        match NonZero::new(ret) {
            Some(e) => Err(HvfCpuError::CreateVcpuFailed(e)),
            None => Ok(HfCpu::new_x64(instance)),
        }
    }
}

unsafe impl Send for Hvf {}
unsafe impl Sync for Hvf {}

/// Implementation of [`Hypervisor::CpuErr`].
#[derive(Debug, Error)]
pub enum HvfCpuError {
    #[error("couldn't create a vCPU ({0:#x})")]
    CreateVcpuFailed(NonZero<hv_return_t>),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't read ID_AA64MMFR0_EL1 ({0:#x})")]
    ReadMmfr0Failed(NonZero<hv_return_t>),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't read ID_AA64MMFR1_EL1 ({0:#x})")]
    ReadMmfr1Failed(NonZero<hv_return_t>),
}

#[cfg(target_arch = "aarch64")]
extern "C" {
    fn os_release(object: *mut std::ffi::c_void);
}
