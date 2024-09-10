// SPDX-License-Identifier: MIT OR Apache-2.0
use self::cpu::HfCpu;
use super::{CpuFeats, Hypervisor};
use crate::vmm::ram::Ram;
use crate::vmm::VmmError;
use hv_sys::{hv_vcpu_create, hv_vm_create, hv_vm_destroy, hv_vm_map};
use std::ffi::c_int;
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
///
/// Fields in this struct need to drop in a correct order.
struct Hvf {
    ram: Ram,
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
            None => Self { ram },
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
}

impl Drop for Hvf {
    fn drop(&mut self) {
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
        use hv_sys::hv_sys_reg_t_HV_SYS_REG_ID_AA64MMFR0_EL1 as HV_SYS_REG_ID_AA64MMFR0_EL1;

        // Load ID_AA64MMFR0_EL1.
        let cpu = self.create_cpu(0)?;
        let reg = cpu
            .read_sys(HV_SYS_REG_ID_AA64MMFR0_EL1)
            .map_err(HvfCpuError::ReadMmfr0Failed)?;

        // FEAT_ExS.
        let feat_exs = match (reg & 0xF00000000000) >> 44 {
            0b0000 => false,
            0b0001 => true,
            _ => unreachable!(),
        };

        // TGran4.
        let tgran4 = match (reg & 0xF0000000) >> 28 {
            0b0000 | 0b0001 => true,
            0b1111 => false,
            _ => unreachable!(),
        };

        // TGran64.
        let tgran64 = match (reg & 0xF000000) >> 24 {
            0b0000 => true,
            0b1111 => false,
            _ => unreachable!(),
        };

        // TGran16.
        let tgran16 = match (reg & 0xF00000) >> 20 {
            0b0000 => false,
            0b0001 | 0b0010 => true,
            _ => unreachable!(),
        };

        // BigEnd.
        let big_end = match (reg & 0xF00) >> 8 {
            0b0000 => false,
            0b0001 => true,
            _ => unreachable!(),
        };

        // BigEndEL0.
        let big_end_el0 = (big_end == false).then(|| match (reg & 0xF0000) >> 16 {
            0b0000 => false,
            0b0001 => true,
            _ => unreachable!(),
        });

        // ASIDBits.
        let asid16 = match (reg & 0xF0) >> 4 {
            0b0000 => false,
            0b0010 => true,
            _ => unreachable!(),
        };

        // PARange.
        let pa_range = (reg & 0xF).try_into().unwrap();

        Ok(CpuFeats {
            feat_exs,
            tgran4,
            tgran64,
            tgran16,
            big_end,
            big_end_el0,
            asid16,
            pa_range,
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
        let ret = unsafe { hv_vcpu_create(&mut instance, &mut exit, std::ptr::null_mut()) };

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

/// Implementation of [`Hypervisor::CpuErr`].
#[derive(Debug, Error)]
pub enum HvfCpuError {
    #[error("couldn't create a vCPU ({0:#x})")]
    CreateVcpuFailed(NonZero<c_int>),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't read ID_AA64MMFR0_EL1 ({0:#x})")]
    ReadMmfr0Failed(NonZero<hv_sys::hv_return_t>),
}
