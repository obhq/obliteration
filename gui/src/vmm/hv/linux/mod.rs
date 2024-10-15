// SPDX-License-Identifier: MIT OR Apache-2.0
use self::cpu::KvmCpu;
use self::ffi::{
    KvmUserspaceMemoryRegion, KVM_API_VERSION, KVM_CAP_MAX_VCPUS, KVM_CHECK_EXTENSION,
    KVM_CREATE_VCPU, KVM_CREATE_VM, KVM_GET_API_VERSION, KVM_GET_VCPU_MMAP_SIZE,
    KVM_SET_USER_MEMORY_REGION,
};
use super::{CpuFeats, Hypervisor};
use crate::vmm::ram::Ram;
use crate::vmm::VmmError;
use libc::{ioctl, mmap, open, MAP_FAILED, MAP_PRIVATE, O_RDWR, PROT_READ, PROT_WRITE};
use std::io::Error;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};
use std::ptr::null_mut;
use std::sync::Mutex;
use thiserror::Error;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod cpu;
mod ffi;
mod run;

pub fn new(cpu: usize, ram: Ram) -> Result<Kvm, VmmError> {
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
    let mr = KvmUserspaceMemoryRegion {
        slot: 0,
        flags: 0,
        guest_phys_addr: 0,
        memory_size: ram.len().get().try_into().unwrap(),
        userspace_addr: (ram.host_addr() as usize).try_into().unwrap(),
    };

    if unsafe { ioctl(vm.as_raw_fd(), KVM_SET_USER_MEMORY_REGION, &mr) } < 0 {
        return Err(VmmError::MapRamFailed(Error::last_os_error()));
    }

    // AArch64 require all CPU to be created before calling KVM_ARM_VCPU_INIT.
    let mut cpus = Vec::with_capacity(cpu);

    for id in 0..cpu {
        let cpu = unsafe { ioctl(vm.as_raw_fd(), KVM_CREATE_VCPU, id) };

        if cpu < 0 {
            return Err(VmmError::CreateCpuFailed(id, Error::last_os_error()));
        }

        cpus.push(Mutex::new(unsafe { OwnedFd::from_raw_fd(cpu) }));
    }

    // Init CPU.
    #[cfg(target_arch = "aarch64")]
    for (i, cpu) in cpus.iter_mut().enumerate() {
        use self::ffi::KVM_ARM_VCPU_INIT;

        let cpu = cpu.get_mut().unwrap();

        if unsafe { ioctl(cpu.as_raw_fd(), KVM_ARM_VCPU_INIT, &preferred_target) < 0 } {
            return Err(VmmError::InitCpuFailed(i, Error::last_os_error()));
        }
    }

    Ok(Kvm {
        feats: load_feats(cpus[0].get_mut().unwrap().as_fd())?,
        cpus,
        vcpu_mmap_size: vcpu_mmap_size.try_into().unwrap(),
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

#[cfg(target_arch = "aarch64")]
fn load_feats(cpu: BorrowedFd) -> Result<CpuFeats, VmmError> {
    use self::ffi::{KvmOneReg, ARM64_SYS_REG, KVM_GET_ONE_REG};
    use crate::vmm::hv::{Mmfr0, Mmfr1, Mmfr2};

    // ID_AA64MMFR0_EL1.
    let mut mmfr0 = Mmfr0::default();
    let mut req = KvmOneReg {
        id: ARM64_SYS_REG(0b11, 0b000, 0b0000, 0b0111, 0b000),
        addr: &mut mmfr0,
    };

    if unsafe { ioctl(cpu.as_raw_fd(), KVM_GET_ONE_REG, &mut req) < 0 } {
        return Err(VmmError::ReadMmfr0Failed(Error::last_os_error()));
    }

    // ID_AA64MMFR1_EL1.
    let mut mmfr1 = Mmfr1::default();
    let mut req = KvmOneReg {
        id: ARM64_SYS_REG(0b11, 0b000, 0b0000, 0b0111, 0b001),
        addr: &mut mmfr1,
    };

    if unsafe { ioctl(cpu.as_raw_fd(), KVM_GET_ONE_REG, &mut req) < 0 } {
        return Err(VmmError::ReadMmfr1Failed(Error::last_os_error()));
    }

    // ID_AA64MMFR2_EL1.
    let mut mmfr2 = Mmfr2::default();
    let mut req = KvmOneReg {
        id: ARM64_SYS_REG(0b11, 0b000, 0b0000, 0b0111, 0b010),
        addr: &mut mmfr2,
    };

    if unsafe { ioctl(cpu.as_raw_fd(), KVM_GET_ONE_REG, &mut req) < 0 } {
        return Err(VmmError::ReadMmfr2Failed(Error::last_os_error()));
    }

    Ok(CpuFeats {
        mmfr0,
        mmfr1,
        mmfr2,
    })
}

#[cfg(target_arch = "x86_64")]
fn load_feats(_: BorrowedFd) -> Result<CpuFeats, VmmError> {
    Ok(CpuFeats {})
}

/// Implementation of [`Hypervisor`] using KVM.
///
/// Fields in this struct need to drop in a correct order (e.g. vm must be dropped before ram).
pub struct Kvm {
    feats: CpuFeats,
    cpus: Vec<Mutex<OwnedFd>>,
    vcpu_mmap_size: usize,
    #[allow(dead_code)]
    vm: OwnedFd,
    ram: Ram,
    #[allow(dead_code)] // kvm are needed by vm.
    kvm: OwnedFd,
}

impl Hypervisor for Kvm {
    type Cpu<'a> = KvmCpu<'a>;
    type CpuErr = KvmCpuError;

    fn cpu_features(&self) -> &CpuFeats {
        &self.feats
    }

    fn ram(&self) -> &Ram {
        &self.ram
    }

    fn ram_mut(&mut self) -> &mut Ram {
        &mut self.ram
    }

    fn create_cpu(&self, id: usize) -> Result<Self::Cpu<'_>, Self::CpuErr> {
        // Get CPU.
        let cpu = self.cpus.get(id).ok_or(KvmCpuError::InvalidId)?;
        let cpu = cpu.try_lock().map_err(|_| KvmCpuError::DuplicatedId)?;

        // Get run context.
        let cx = unsafe {
            mmap(
                null_mut(),
                self.vcpu_mmap_size,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE,
                cpu.as_raw_fd(),
                0,
            )
        };

        if cx == MAP_FAILED {
            return Err(KvmCpuError::GetKvmRunFailed(Error::last_os_error()));
        }

        Ok(unsafe { KvmCpu::new(cpu, cx.cast(), self.vcpu_mmap_size) })
    }
}

/// Implementation of [`Hypervisor::CpuErr`].
#[derive(Debug, Error)]
pub enum KvmCpuError {
    #[error("invalid CPU identifier")]
    InvalidId,

    #[error("CPU identifier currently in use")]
    DuplicatedId,

    #[error("couldn't get a pointer to kvm_run")]
    GetKvmRunFailed(#[source] std::io::Error),
}
