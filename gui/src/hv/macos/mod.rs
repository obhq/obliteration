// SPDX-License-Identifier: MIT OR Apache-2.0
use self::cpu::HvfCpu;
use self::mapper::HvfMapper;
use super::{CpuFeats, Hypervisor, Ram};
use applevisor_sys::hv_feature_reg_t::{
    HV_FEATURE_REG_ID_AA64MMFR0_EL1, HV_FEATURE_REG_ID_AA64MMFR1_EL1,
    HV_FEATURE_REG_ID_AA64MMFR2_EL1,
};
use applevisor_sys::{
    hv_feature_reg_t, hv_return_t, hv_vcpu_config_create, hv_vcpu_config_get_feature_reg,
    hv_vcpu_config_t, hv_vcpu_create, hv_vcpu_set_trap_debug_exceptions, hv_vm_create,
    hv_vm_destroy, hv_vm_get_max_vcpu_count,
};
use std::num::NonZero;
use std::ptr::{null, null_mut};
use thiserror::Error;

mod cpu;
mod mapper;

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
) -> Result<Hvf, HvfError> {
    // Create RAM.
    let ram = Ram::new(ram_size, ram_block, HvfMapper).map_err(HvfError::CreateRamFailed)?;

    // Create a VM.
    let ret = unsafe { hv_vm_create(null_mut()) };
    let mut hv = match NonZero::new(ret) {
        Some(ret) => return Err(HvfError::CreateVmFailed(ret)),
        None => Hvf {
            ram,
            debug,
            cpu_config: unsafe { hv_vcpu_config_create() },
            feats: CpuFeats::default(),
        },
    };

    // Get max vCPU count.
    let mut max = 0;
    let ret = hv_vm_get_max_vcpu_count(&mut max);

    if let Some(v) = NonZero::new(ret) {
        return Err(HvfError::GetMaxCpuFailed(v));
    } else if usize::try_from(max).unwrap() < cpu {
        return Err(HvfError::MaxCpuTooLow(cpu));
    }

    // Load PE features.
    hv.feats.mmfr0 = hv
        .read_feature_reg(HV_FEATURE_REG_ID_AA64MMFR0_EL1)
        .map_err(HvfError::ReadMmfr0Failed)?
        .into();
    hv.feats.mmfr1 = hv
        .read_feature_reg(HV_FEATURE_REG_ID_AA64MMFR1_EL1)
        .map_err(HvfError::ReadMmfr1Failed)?
        .into();
    hv.feats.mmfr2 = hv
        .read_feature_reg(HV_FEATURE_REG_ID_AA64MMFR2_EL1)
        .map_err(HvfError::ReadMmfr2Failed)?
        .into();

    Ok(hv)
}

/// Implementation of [`Hypervisor`] using Hypervisor Framework.
pub struct Hvf {
    ram: Ram<HvfMapper>,
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
    type Mapper = HvfMapper;
    type Cpu<'a> = HvfCpu<'a>;
    type CpuErr = HvfCpuError;

    fn cpu_features(&self) -> &CpuFeats {
        &self.feats
    }

    fn ram(&self) -> &Ram<Self::Mapper> {
        &self.ram
    }

    fn ram_mut(&mut self) -> &mut Ram<Self::Mapper> {
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

/// Represents an error when [`Hvf`] fails to initialize.
#[derive(Debug, Error)]
pub enum HvfError {
    #[error("couldn't create a RAM")]
    CreateRamFailed(#[source] std::io::Error),

    #[error("couldn't create a VM ({0:#x})")]
    CreateVmFailed(NonZero<hv_return_t>),

    #[error("couldn't get maximum number of vCPU for a VM")]
    GetMaxCpuFailed(NonZero<hv_return_t>),

    #[error("your OS does not support {0} vCPU on a VM")]
    MaxCpuTooLow(usize),

    #[error("couldn't read ID_AA64MMFR0_EL1 ({0:#x})")]
    ReadMmfr0Failed(NonZero<hv_return_t>),

    #[error("couldn't read ID_AA64MMFR1_EL1 ({0:#x})")]
    ReadMmfr1Failed(NonZero<hv_return_t>),

    #[error("couldn't read ID_AA64MMFR2_EL1 ({0:#x})")]
    ReadMmfr2Failed(NonZero<hv_return_t>),
}

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
