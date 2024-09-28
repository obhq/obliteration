// SPDX-License-Identifier: MIT OR Apache-2.0
use self::cpu::KvmCpu;
use self::ffi::{
    kvm_set_user_memory_region, KVM_API_VERSION, KVM_CAP_MAX_VCPUS, KVM_CHECK_EXTENSION,
    KVM_CREATE_VCPU, KVM_CREATE_VM, KVM_GET_API_VERSION, KVM_GET_VCPU_MMAP_SIZE,
};
use super::{CpuFeats, Hypervisor};
use crate::vmm::ram::Ram;
use crate::vmm::VmmError;
use libc::{ioctl, mmap, open, MAP_FAILED, MAP_PRIVATE, O_RDWR, PROT_READ, PROT_WRITE};
use std::io::Error;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};
use std::ptr::null_mut;
use thiserror::Error;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod cpu;
mod ffi;
mod run;

pub fn new(cpu: usize, ram: Ram) -> Result<impl Hypervisor, VmmError> {
    // Open KVM device.
    let kvm = unsafe { open(b"/dev/kvm\0".as_ptr().cast(), O_RDWR) };

    if kvm < 0 {
        return Err(VmmError::OpenKvmFailed(Error::last_os_error()));
    }

    // Check KVM version.
    let kvm = unsafe { OwnedFd::from_raw_fd(kvm) };
    let version = unsafe { ioctl(kvm.as_raw_fd(), KVM_GET_API_VERSION) };

    if version < 0 {
        return Err(VmmError::GetKvmVersionFailed(Error::last_os_error()));
    } else if version != KVM_API_VERSION {
        return Err(VmmError::KvmVersionMismatched);
    }

    // Check max CPU.
    let max = unsafe { ioctl(kvm.as_raw_fd(), KVM_CHECK_EXTENSION, KVM_CAP_MAX_VCPUS) };

    if max < 0 {
        return Err(VmmError::GetMaxCpuFailed(Error::last_os_error()));
    } else if TryInto::<usize>::try_into(max).unwrap() < cpu {
        return Err(VmmError::MaxCpuTooLow);
    }

    // On AArch64 we need KVM_SET_ONE_REG and KVM_GET_ONE_REG.
    #[cfg(target_arch = "aarch64")]
    if unsafe {
        ioctl(
            kvm.as_raw_fd(),
            KVM_CHECK_EXTENSION,
            self::ffi::KVM_CAP_ONE_REG,
        ) <= 0
    } {
        return Err(VmmError::NoKvmOneReg);
    }

    // Get size of CPU context.
    let vcpu_mmap_size = unsafe { ioctl(kvm.as_raw_fd(), KVM_GET_VCPU_MMAP_SIZE, 0) };

    if vcpu_mmap_size < 0 {
        return Err(VmmError::GetMmapSizeFailed(Error::last_os_error()));
    }

    // Create a VM.
    let vm = create_vm(kvm.as_fd())?;
    #[cfg(target_arch = "aarch64")]
    let preferred_target = unsafe {
        let mut v: self::ffi::KvmVcpuInit = std::mem::zeroed();

        if ioctl(vm.as_raw_fd(), self::ffi::KVM_ARM_PREFERRED_TARGET, &mut v) < 0 {
            return Err(VmmError::GetPreferredTargetFailed(Error::last_os_error()));
        }

        v
    };

    // Set RAM.
    let slot = 0;
    let len = ram.len().try_into().unwrap();
    let mem = ram.host_addr().cast_mut().cast();

    match unsafe { kvm_set_user_memory_region(vm.as_raw_fd(), slot, 0, len, mem) } {
        0 => {}
        v => return Err(VmmError::MapRamFailed(Error::from_raw_os_error(v))),
    }

    Ok(Kvm {
        vcpu_mmap_size: vcpu_mmap_size.try_into().unwrap(),
        #[cfg(target_arch = "aarch64")]
        preferred_target,
        vm,
        ram,
        kvm,
    })
}

#[cfg(target_arch = "aarch64")]
fn create_vm(kvm: BorrowedFd) -> Result<OwnedFd, VmmError> {
    use self::ffi::{KVM_CAP_ARM_VM_IPA_SIZE, KVM_VM_TYPE_ARM_IPA_SIZE};

    // Check KVM_CAP_ARM_VM_IPA_SIZE. We cannot use default machine type on AArch64 otherwise
    // KVM_CREATE_VM will fails on Apple M1 due to the default IPA size is 40-bits, which M1 does
    // not support.
    let limit = unsafe {
        ioctl(
            kvm.as_raw_fd(),
            KVM_CHECK_EXTENSION,
            KVM_CAP_ARM_VM_IPA_SIZE,
        )
    };

    if limit <= 0 {
        return Err(VmmError::NoVmIpaSize);
    } else if limit < 36 {
        return Err(VmmError::PhysicalAddressTooSmall);
    }

    // Create a VM.
    let vm = unsafe { ioctl(kvm.as_raw_fd(), KVM_CREATE_VM, KVM_VM_TYPE_ARM_IPA_SIZE(36)) };

    if vm < 0 {
        Err(VmmError::CreateVmFailed(Error::last_os_error()))
    } else {
        Ok(unsafe { OwnedFd::from_raw_fd(vm) })
    }
}

#[cfg(target_arch = "x86_64")]
fn create_vm(kvm: BorrowedFd) -> Result<OwnedFd, VmmError> {
    let vm = unsafe { ioctl(kvm.as_raw_fd(), KVM_CREATE_VM, 0) };

    if vm < 0 {
        Err(VmmError::CreateVmFailed(Error::last_os_error()))
    } else {
        Ok(unsafe { OwnedFd::from_raw_fd(vm) })
    }
}

/// Implementation of [`Hypervisor`] using KVM.
///
/// Fields in this struct need to drop in a correct order (e.g. vm must be dropped before ram).
struct Kvm {
    vcpu_mmap_size: usize,
    #[cfg(target_arch = "aarch64")]
    preferred_target: self::ffi::KvmVcpuInit,
    vm: OwnedFd,
    ram: Ram,
    #[allow(dead_code)] // kvm are needed by vm.
    kvm: OwnedFd,
}

impl Kvm {
    #[cfg(target_arch = "aarch64")]
    fn create_cpu(&self, id: usize) -> Result<OwnedFd, KvmCpuError> {
        use self::ffi::KVM_ARM_VCPU_INIT;

        // Create CPU.
        let cpu = unsafe { ioctl(self.vm.as_raw_fd(), KVM_CREATE_VCPU, id) };

        if cpu < 0 {
            return Err(KvmCpuError::CreateCpuFailed(Error::last_os_error()));
        }

        // Init CPU.
        let cpu = unsafe { OwnedFd::from_raw_fd(cpu) };

        if unsafe { ioctl(cpu.as_raw_fd(), KVM_ARM_VCPU_INIT, &self.preferred_target) < 0 } {
            return Err(KvmCpuError::InitCpuFailed(Error::last_os_error()));
        }

        Ok(cpu)
    }

    #[cfg(target_arch = "x86_64")]
    fn create_cpu(&self, id: usize) -> Result<OwnedFd, KvmCpuError> {
        let cpu = unsafe { ioctl(self.vm.as_raw_fd(), KVM_CREATE_VCPU, id) };

        if cpu < 0 {
            Err(KvmCpuError::CreateCpuFailed(Error::last_os_error()))
        } else {
            Ok(unsafe { OwnedFd::from_raw_fd(cpu) })
        }
    }
}

impl Hypervisor for Kvm {
    type Cpu<'a> = KvmCpu<'a>;
    type CpuErr = KvmCpuError;

    #[cfg(target_arch = "aarch64")]
    fn cpu_features(&mut self) -> Result<CpuFeats, Self::CpuErr> {
        use self::ffi::{KvmOneReg, ARM64_SYS_REG, KVM_GET_ONE_REG};
        use crate::vmm::hv::{Mmfr0, Mmfr1, Mmfr2};
        use std::arch::asm;

        // ID_AA64MMFR0_EL1.
        let cpu = self.create_cpu(0)?;
        let mut mmfr0 = Mmfr0::default();
        let mut req = KvmOneReg {
            id: ARM64_SYS_REG(0b11, 0b000, 0b0000, 0b0111, 0b000),
            addr: &mut mmfr0,
        };

        if unsafe { ioctl(cpu.as_raw_fd(), KVM_GET_ONE_REG, &mut req) < 0 } {
            return Err(KvmCpuError::ReadMmfr0Failed(Error::last_os_error()));
        }

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
            mmfr0,
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
        let vcpu = self.create_cpu(id)?;
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
    #[error("couldn't create vCPU")]
    CreateCpuFailed(#[source] std::io::Error),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't initialize vCPU")]
    InitCpuFailed(#[source] std::io::Error),

    #[error("couldn't get a pointer to kvm_run")]
    GetKvmRunFailed(#[source] std::io::Error),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't read ID_AA64MMFR0_EL1")]
    ReadMmfr0Failed(#[source] std::io::Error),
}
