// SPDX-License-Identifier: MIT OR Apache-2.0
use self::cpu::HvfCpu;
use super::{CpuFeats, Hypervisor};
use crate::vmm::ram::Ram;
use crate::vmm::VmmError;
use applevisor_sys::hv_feature_reg_t::{
    HV_FEATURE_REG_ID_AA64MMFR0_EL1, HV_FEATURE_REG_ID_AA64MMFR1_EL1,
    HV_FEATURE_REG_ID_AA64MMFR2_EL1,
};
use applevisor_sys::{
    hv_feature_reg_t, hv_return_t, hv_vcpu_config_create, hv_vcpu_config_get_feature_reg,
    hv_vcpu_config_t, hv_vcpu_create, hv_vcpu_set_trap_debug_exceptions, hv_vm_create,
    hv_vm_destroy, hv_vm_map, HV_MEMORY_EXEC, HV_MEMORY_READ, HV_MEMORY_WRITE,
};
use std::num::NonZero;
use std::ptr::{null, null_mut};
use thiserror::Error;

mod cpu;

pub fn new(_: usize, ram: Ram, debug: bool) -> Result<Hvf, VmmError> {
    // Create a VM.
    let ret = unsafe { hv_vm_create(null_mut()) };
    let mut hv = match NonZero::new(ret) {
        Some(ret) => return Err(VmmError::CreateVmFailed(ret)),
        None => Hvf {
            ram,
            debug,
            cpu_config: unsafe { hv_vcpu_config_create() },
            feats: CpuFeats::default(),
        },
    };

    // Load PE features.
    hv.feats.mmfr0 = hv
        .read_feature_reg(HV_FEATURE_REG_ID_AA64MMFR0_EL1)
        .map_err(VmmError::ReadMmfr0Failed)?
        .into();
    hv.feats.mmfr1 = hv
        .read_feature_reg(HV_FEATURE_REG_ID_AA64MMFR1_EL1)
        .map_err(VmmError::ReadMmfr1Failed)?
        .into();
    hv.feats.mmfr2 = hv
        .read_feature_reg(HV_FEATURE_REG_ID_AA64MMFR2_EL1)
        .map_err(VmmError::ReadMmfr2Failed)?
        .into();

    // Set RAM.
    let host = hv.ram.host_addr().cast_mut().cast();
    let len = hv.ram.len().get().try_into().unwrap();
    let ret = unsafe {
        hv_vm_map(
            host,
            0,
            len,
            HV_MEMORY_READ | HV_MEMORY_WRITE | HV_MEMORY_EXEC,
        )
    };

    match NonZero::new(ret) {
        Some(ret) => Err(VmmError::MapRamFailed(ret)),
        None => Ok(hv),
    }
}

/// Implementation of [`Hypervisor`] using Hypervisor Framework.
pub struct Hvf {
    ram: Ram,
    debug: bool,
    cpu_config: hv_vcpu_config_t,
    feats: CpuFeats,
}

impl Hvf {
    fn read_feature_reg(&self, reg: hv_feature_reg_t) -> Result<u64, NonZero<hv_return_t>> {
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
        unsafe { os_release(self.cpu_config.cast()) };

        // Destroy VM.
        let status = unsafe { hv_vm_destroy() };

        if status != 0 {
            panic!("hv_vm_destroy() fails with {status:#x}");
        }
    }
}

impl Hypervisor for Hvf {
    type Cpu<'a> = HvfCpu<'a>;
    type CpuErr = HvfCpuError;

    fn cpu_features(&self) -> &CpuFeats {
        &self.feats
    }

    fn ram(&self) -> &Ram {
        &self.ram
    }

    fn ram_mut(&mut self) -> &mut Ram {
        &mut self.ram
    }

    fn create_cpu(&self, _: usize) -> Result<Self::Cpu<'_>, Self::CpuErr> {
        // Create vCPU.
        let mut instance = 0;
        let mut exit = null();
        let ret = unsafe { hv_vcpu_create(&mut instance, &mut exit, self.cpu_config) };
        let cpu = match NonZero::new(ret) {
            Some(e) => return Err(HvfCpuError::CreateVcpuFailed(e)),
            None => HvfCpu::new(instance, exit),
        };

        // Trap debug exception.
        if self.debug {
            let ret = unsafe { hv_vcpu_set_trap_debug_exceptions(instance, true) };

            if let Some(e) = NonZero::new(ret) {
                return Err(HvfCpuError::EnableDebugFailed(e));
            }
        }

        Ok(cpu)
    }
}

unsafe impl Send for Hvf {}
unsafe impl Sync for Hvf {}

/// Implementation of [`Hypervisor::CpuErr`].
#[derive(Debug, Error)]
pub enum HvfCpuError {
    #[error("couldn't create a vCPU ({0:#x})")]
    CreateVcpuFailed(NonZero<hv_return_t>),

    #[error("couldn't enable debug on a vCPU ({0:#x})")]
    EnableDebugFailed(NonZero<hv_return_t>),
}

extern "C" {
    fn os_release(object: *mut std::ffi::c_void);
}
