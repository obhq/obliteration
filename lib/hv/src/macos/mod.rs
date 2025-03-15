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

/// `page_size` is a page size on the VM. This value will be used as a block size if it is larger
/// than page size on the host otherwise block size will be page size on the host.
///
/// `ram_size` must be multiply by the block size calculated from the above.
///
/// # Panics
/// If `page_size` is not power of two.
pub fn new(
    cpu: usize,
    ram_size: NonZero<usize>,
    page_size: NonZero<usize>,
    debug: bool,
) -> Result<impl Hypervisor, HvError> {
    // Create RAM.
    let ram = Ram::new(page_size, ram_size, HvfMapper)?;

    // Create a VM.
    let ret = unsafe { hv_vm_create(null_mut()) };
    let mut hv = match NonZero::new(ret) {
        Some(ret) => return Err(HvError::CreateVmFailed(ret)),
        None => Hvf {
            ram,
            debug,
            cpu_config: unsafe { hv_vcpu_config_create() },
            feats: CpuFeats::default(),
        },
    };

    // Get max vCPU count.
    let mut max = 0;
    let ret = unsafe { hv_vm_get_max_vcpu_count(&mut max) };

    if let Some(v) = NonZero::new(ret) {
        return Err(HvError::GetMaxCpuFailed(v));
    } else if usize::try_from(max).unwrap() < cpu {
        return Err(HvError::MaxCpuTooLow(cpu));
    }

    // Load PE features.
    hv.feats.mmfr0 = hv
        .read_feature_reg(HV_FEATURE_REG_ID_AA64MMFR0_EL1)
        .map_err(HvError::ReadMmfr0Failed)?
        .into();
    hv.feats.mmfr1 = hv
        .read_feature_reg(HV_FEATURE_REG_ID_AA64MMFR1_EL1)
        .map_err(HvError::ReadMmfr1Failed)?
        .into();
    hv.feats.mmfr2 = hv
        .read_feature_reg(HV_FEATURE_REG_ID_AA64MMFR2_EL1)
        .map_err(HvError::ReadMmfr2Failed)?
        .into();

    // Check if PE support VM page size.
    match page_size.get() {
        0x4000 => {
            if hv.feats.mmfr0.t_gran16() == 0b0000 {
                return Err(HvError::PageSizeNotSupported(page_size));
            }
        }
        _ => todo!(),
    }

    // Check if PE support RAM size.
    let max = match hv.feats.mmfr0.pa_range() {
        0b0000 => 1024usize.pow(3) * 4,
        0b0001 => 1024usize.pow(3) * 64,
        0b0010 => 1024usize.pow(4),
        0b0011 => 1024usize.pow(4) * 4,
        0b0100 => 1024usize.pow(4) * 16,
        0b0101 => 1024usize.pow(4) * 256,
        0b0110 => 1024usize.pow(5) * 4,
        0b0111 => 1024usize.pow(5) * 64,
        _ => unreachable!(),
    };

    if ram_size.get() > max {
        return Err(HvError::RamSizeNotSupported(ram_size));
    }

    Ok(hv)
}

/// Implementation of [`Hypervisor`] using Hypervisor Framework.
struct Hvf {
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

/// Represents an error when operation on Hypervisor Framework fails.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum HvError {
    #[error("couldn't get host page size")]
    GetHostPageSize(#[source] std::io::Error),

    #[error("size of RAM is not valid")]
    InvalidRamSize,

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

    #[error("your CPU does not support {0:#x} page size on a VM")]
    PageSizeNotSupported(NonZero<usize>),

    #[error("your CPU does not support {0:#x} bytes of RAM on a VM")]
    RamSizeNotSupported(NonZero<usize>),
}

/// Implementation of [`Hypervisor::CpuErr`].
#[derive(Debug, Error)]
pub enum HvfCpuError {
    #[error("couldn't create a vCPU ({0:#x})")]
    CreateVcpuFailed(NonZero<hv_return_t>),

    #[error("couldn't enable debug on a vCPU ({0:#x})")]
    EnableDebugFailed(NonZero<hv_return_t>),
}

unsafe extern "C" {
    fn os_release(object: *mut std::ffi::c_void);
}
