// SPDX-License-Identifier: MIT OR Apache-2.0
use self::cpu::KvmCpu;
use self::ffi::{
    kvm_check_extension, kvm_check_version, kvm_create_vcpu, kvm_create_vm, kvm_get_vcpu_mmap_size,
    kvm_max_vcpus, kvm_set_user_memory_region,
};
use super::{CpuFeats, Hypervisor};
use crate::vmm::ram::Ram;
use crate::vmm::VmmError;
use libc::{mmap, open, MAP_FAILED, MAP_PRIVATE, O_RDWR, PROT_READ, PROT_WRITE};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::ptr::null_mut;
use thiserror::Error;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod cpu;
mod ffi;
mod run;

pub fn new(cpu: usize, ram: Ram) -> Result<impl Hypervisor, VmmError> {
    use std::io::Error;

    // Open KVM device.
    let kvm = unsafe { open(b"/dev/kvm\0".as_ptr().cast(), O_RDWR) };

    if kvm < 0 {
        return Err(VmmError::OpenKvmFailed(Error::last_os_error()));
    }

    // Check KVM version.
    let kvm = unsafe { OwnedFd::from_raw_fd(kvm) };
    let mut compat = false;

    match unsafe { kvm_check_version(kvm.as_raw_fd(), &mut compat) } {
        0 if !compat => {
            return Err(VmmError::KvmVersionMismatched);
        }
        0 => {}
        v => return Err(VmmError::GetKvmVersionFailed(Error::from_raw_os_error(v))),
    }

    // Check max CPU.
    let mut max = 0;

    match unsafe { kvm_max_vcpus(kvm.as_raw_fd(), &mut max) } {
        0 => {}
        v => {
            return Err(VmmError::GetMaxCpuFailed(Error::from_raw_os_error(v)));
        }
    }

    if max < cpu {
        return Err(VmmError::MaxCpuTooLow);
    }

    // Check KVM_CAP_ONE_REG. KVM_SET_ONE_REG and KVM_GET_ONE_REG are the only API that support all
    // architectures.
    if unsafe { !kvm_check_extension(kvm.as_raw_fd(), 70) } {
        return Err(VmmError::NoKvmOneReg);
    }

    // Get size of CPU context.
    let vcpu_mmap_size = match unsafe { kvm_get_vcpu_mmap_size(kvm.as_raw_fd()) } {
        size @ 0.. => size as usize,
        _ => return Err(VmmError::GetMmapSizeFailed(Error::last_os_error())),
    };

    // Create a VM.
    let mut vm = -1;

    match unsafe { kvm_create_vm(kvm.as_raw_fd(), &mut vm) } {
        0 => {}
        v => return Err(VmmError::CreateVmFailed(Error::from_raw_os_error(v))),
    }

    // Set RAM.
    let vm = unsafe { OwnedFd::from_raw_fd(vm) };
    let slot = 0;
    let len = ram.len().try_into().unwrap();
    let mem = ram.host_addr().cast_mut().cast();

    match unsafe { kvm_set_user_memory_region(vm.as_raw_fd(), slot, 0, len, mem) } {
        0 => {}
        v => return Err(VmmError::MapRamFailed(Error::from_raw_os_error(v))),
    }

    Ok(Kvm {
        vcpu_mmap_size,
        vm,
        ram,
        kvm,
    })
}

/// Implementation of [`Hypervisor`] using KVM.
///
/// Fields in this struct need to drop in a correct order (e.g. vm must be dropped before ram).
struct Kvm {
    vcpu_mmap_size: usize,
    vm: OwnedFd,
    ram: Ram,
    #[allow(dead_code)] // kvm are needed by vm.
    kvm: OwnedFd,
}

impl Hypervisor for Kvm {
    type Cpu<'a> = KvmCpu<'a>;
    type CpuErr = KvmCpuError;

    #[cfg(target_arch = "aarch64")]
    fn cpu_features(&mut self) -> Result<CpuFeats, Self::CpuErr> {
        // See https://www.kernel.org/doc/html/latest/arch/arm64/cpu-feature-registers.html for the
        // reason why we can access *_EL1 registers from a user space.
        use crate::vmm::hv::{Mmfr0, Mmfr1, Mmfr2};
        use std::arch::asm;

        // ID_AA64MMFR0_EL1.
        let mut mmfr0;

        unsafe {
            asm!(
                "mrs {v}, ID_AA64MMFR0_EL1",
                v = out(reg) mmfr0,
                options(pure, nomem, preserves_flags, nostack)
            )
        };

        // ID_AA64MMFR1_EL1.
        let mut mmfr1;

        unsafe {
            asm!(
                "mrs {v}, ID_AA64MMFR1_EL1",
                v = out(reg) mmfr1,
                options(pure, nomem, preserves_flags, nostack)
            )
        };

        // ID_AA64MMFR2_EL1.
        let mut mmfr2;

        unsafe {
            asm!(
                "mrs {v}, ID_AA64MMFR2_EL1",
                v = out(reg) mmfr2,
                options(pure, nomem, preserves_flags, nostack)
            )
        };

        Ok(CpuFeats {
            mmfr0: Mmfr0::from_bits(mmfr0),
            mmfr1: Mmfr1::from_bits(mmfr1),
            mmfr2: Mmfr2::from_bits(mmfr2),
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

    fn create_cpu(&self, id: usize) -> Result<Self::Cpu<'_>, Self::CpuErr> {
        use std::io::Error;

        // Create vCPU.
        let id = id.try_into().unwrap();
        let mut vcpu = -1;
        let vcpu = match unsafe { kvm_create_vcpu(self.vm.as_raw_fd(), id, &mut vcpu) } {
            0 => unsafe { OwnedFd::from_raw_fd(vcpu) },
            v => return Err(KvmCpuError::CreateVcpuFailed(Error::from_raw_os_error(v))),
        };

        // Get kvm_run.
        let cx = unsafe {
            mmap(
                null_mut(),
                self.vcpu_mmap_size,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE,
                vcpu.as_raw_fd(),
                0,
            )
        };

        if cx == MAP_FAILED {
            return Err(KvmCpuError::GetKvmRunFailed(Error::last_os_error()));
        }

        Ok(unsafe { KvmCpu::new(vcpu, cx.cast(), self.vcpu_mmap_size) })
    }
}

/// Implementation of [`Hypervisor::CpuErr`].
#[derive(Debug, Error)]
pub enum KvmCpuError {
    #[error("failed to create vcpu")]
    CreateVcpuFailed(#[source] std::io::Error),

    #[error("couldn't get a pointer to kvm_run")]
    GetKvmRunFailed(#[source] std::io::Error),
}
