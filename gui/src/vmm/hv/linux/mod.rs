// SPDX-License-Identifier: MIT OR Apache-2.0
use self::cpu::KvmCpu;
use self::ffi::{
    KvmGuestDebug, KvmUserspaceMemoryRegion, KVM_API_VERSION, KVM_CAP_MAX_VCPUS,
    KVM_CAP_SET_GUEST_DEBUG, KVM_CHECK_EXTENSION, KVM_CREATE_VCPU, KVM_CREATE_VM,
    KVM_GET_API_VERSION, KVM_GET_VCPU_MMAP_SIZE, KVM_GUESTDBG_ENABLE, KVM_GUESTDBG_USE_SW_BP,
    KVM_SET_GUEST_DEBUG, KVM_SET_USER_MEMORY_REGION,
};
use self::mapper::KvmMapper;
use super::{CpuFeats, Hypervisor, Ram};
use libc::{ioctl, mmap, open, MAP_FAILED, MAP_PRIVATE, O_RDWR, PROT_READ, PROT_WRITE};
use std::ffi::{c_int, c_uint};
use std::io::Error;
use std::mem::zeroed;
use std::num::NonZero;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};
use std::ptr::null_mut;
use std::sync::Mutex;
use thiserror::Error;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod cpu;
mod ffi;
mod mapper;
mod run;

/// Panics
/// If `ram_size` is not multiply by `ram_block`.
///
/// # Safety
/// `ram_block` must be greater or equal host page size.
pub fn new(
    cpu: usize,
    ram_size: NonZero<usize>,
    ram_block: NonZero<usize>,
    debug: bool,
) -> Result<Kvm, KvmError> {
    // Create RAM.
    let ram = Ram::new(ram_size, ram_block, KvmMapper).map_err(KvmError::CreateRamFailed)?;

    // Open KVM device.
    let kvm = unsafe { open("/dev/kvm\0".as_ptr().cast(), O_RDWR) };

    if kvm < 0 {
        return Err(KvmError::OpenKvmFailed(Error::last_os_error()));
    }

    // Check KVM version.
    let kvm = unsafe { OwnedFd::from_raw_fd(kvm) };
    let version = unsafe { ioctl(kvm.as_raw_fd(), KVM_GET_API_VERSION) };

    if version < 0 {
        return Err(KvmError::GetKvmVersionFailed(Error::last_os_error()));
    } else if version != KVM_API_VERSION {
        return Err(KvmError::KvmVersionMismatched);
    }

    // Check max CPU.
    let max = get_ext(kvm.as_fd(), KVM_CAP_MAX_VCPUS).map_err(KvmError::GetMaxCpuFailed)?;

    if usize::try_from(max).unwrap() < cpu {
        return Err(KvmError::MaxCpuTooLow(cpu));
    }

    // On AArch64 we need KVM_SET_ONE_REG and KVM_GET_ONE_REG.
    #[cfg(target_arch = "aarch64")]
    if !get_ext(kvm.as_fd(), self::ffi::KVM_CAP_ONE_REG).is_ok_and(|v| v != 0) {
        return Err(KvmError::NoKvmOneReg);
    }

    // On x86 we need KVM_GET_SUPPORTED_CPUID.
    #[cfg(target_arch = "x86_64")]
    let cpuid = if !get_ext(kvm.as_fd(), self::ffi::KVM_CAP_EXT_CPUID).is_ok_and(|v| v != 0) {
        return Err(KvmError::NoKvmExtCpuid);
    } else {
        use self::ffi::{KvmCpuid2, KvmCpuidEntry2, KVM_GET_SUPPORTED_CPUID};
        use libc::E2BIG;
        use std::alloc::{handle_alloc_error, Layout};

        let layout = Layout::from_size_align(8, 4).unwrap();
        let mut count = 1;

        loop {
            // Allocate kvm_cpuid2.
            let layout = layout
                .extend(Layout::array::<KvmCpuidEntry2>(count).unwrap())
                .map(|v| v.0.pad_to_align())
                .unwrap();
            let data = unsafe { std::alloc::alloc_zeroed(layout) };

            if data.is_null() {
                handle_alloc_error(layout);
            }

            unsafe { data.cast::<u32>().write(count.try_into().unwrap()) };

            // Execute KVM_GET_SUPPORTED_CPUID.
            let e = if unsafe { ioctl(kvm.as_raw_fd(), KVM_GET_SUPPORTED_CPUID, data) } < 0 {
                Some(Error::last_os_error())
            } else {
                None
            };

            // Wrap data in a box. Pointer casting here may looks weird but it is how unsized type
            // works in Rust. See https://stackoverflow.com/a/64121094 for more details.
            let data = std::ptr::slice_from_raw_parts_mut(data.cast::<KvmCpuidEntry2>(), count);
            let data = unsafe { Box::from_raw(data as *mut KvmCpuid2) };
            let e = match e {
                Some(v) => v,
                None => break data,
            };

            // Check if E2BIG.
            if e.raw_os_error().unwrap() != E2BIG {
                return Err(KvmError::GetSupportedCpuidFailed(e));
            }

            count += 1;
        }
    };

    // Check if debug supported.
    if debug && !get_ext(kvm.as_fd(), KVM_CAP_SET_GUEST_DEBUG).is_ok_and(|v| v != 0) {
        return Err(KvmError::DebugNotSupported);
    }

    // Get size of CPU context.
    let vcpu_mmap_size = unsafe { ioctl(kvm.as_raw_fd(), KVM_GET_VCPU_MMAP_SIZE, 0) };

    if vcpu_mmap_size < 0 {
        return Err(KvmError::GetMmapSizeFailed(Error::last_os_error()));
    }

    // Create a VM.
    let vm = create_vm(kvm.as_fd())?;
    #[cfg(target_arch = "aarch64")]
    let preferred_target = unsafe {
        let mut v: self::ffi::KvmVcpuInit = std::mem::zeroed();

        if ioctl(vm.as_raw_fd(), self::ffi::KVM_ARM_PREFERRED_TARGET, &mut v) < 0 {
            return Err(KvmError::GetPreferredTargetFailed(Error::last_os_error()));
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
        return Err(KvmError::MapRamFailed(Error::last_os_error()));
    }

    // AArch64 require all CPU to be created before calling KVM_ARM_VCPU_INIT.
    let mut cpus = Vec::with_capacity(cpu);

    for id in 0..cpu {
        let cpu = unsafe { ioctl(vm.as_raw_fd(), KVM_CREATE_VCPU, id) };

        if cpu < 0 {
            return Err(KvmError::CreateCpuFailed(id, Error::last_os_error()));
        }

        cpus.push(Mutex::new(unsafe { OwnedFd::from_raw_fd(cpu) }));
    }

    // Init CPU.
    for (i, cpu) in cpus.iter_mut().enumerate() {
        let cpu = cpu.get_mut().unwrap();

        #[cfg(target_arch = "aarch64")]
        if unsafe {
            ioctl(
                cpu.as_raw_fd(),
                self::ffi::KVM_ARM_VCPU_INIT,
                &preferred_target,
            ) < 0
        } {
            return Err(KvmError::InitCpuFailed(i, Error::last_os_error()));
        }

        #[cfg(target_arch = "x86_64")]
        if unsafe {
            ioctl(
                cpu.as_raw_fd(),
                self::ffi::KVM_SET_CPUID2,
                cpuid.as_ref() as *const self::ffi::KvmCpuid2 as *const u8,
            ) < 0
        } {
            return Err(KvmError::SetCpuidFailed(i, Error::last_os_error()));
        }

        if debug {
            let arg = KvmGuestDebug {
                control: KVM_GUESTDBG_ENABLE | KVM_GUESTDBG_USE_SW_BP,
                pad: 0,
                arch: unsafe { zeroed() },
            };

            if unsafe { ioctl(cpu.as_raw_fd(), KVM_SET_GUEST_DEBUG, &arg) } < 0 {
                return Err(KvmError::EnableDebugFailed(i, Error::last_os_error()));
            }
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
fn create_vm(kvm: BorrowedFd) -> Result<OwnedFd, KvmError> {
    use self::ffi::{KVM_CAP_ARM_VM_IPA_SIZE, KVM_VM_TYPE_ARM_IPA_SIZE};

    // Check KVM_CAP_ARM_VM_IPA_SIZE. We cannot use default machine type on AArch64 otherwise
    // KVM_CREATE_VM will fails on Apple M1 due to the default IPA size is 40-bits, which M1 does
    // not support.
    let limit = get_ext(kvm.as_fd(), KVM_CAP_ARM_VM_IPA_SIZE).unwrap_or(0);

    if limit == 0 {
        return Err(KvmError::NoVmIpaSize);
    } else if limit < 36 {
        return Err(KvmError::PhysicalAddressTooSmall);
    }

    // Create a VM.
    let vm = unsafe { ioctl(kvm.as_raw_fd(), KVM_CREATE_VM, KVM_VM_TYPE_ARM_IPA_SIZE(36)) };

    if vm < 0 {
        Err(KvmError::CreateVmFailed(Error::last_os_error()))
    } else {
        Ok(unsafe { OwnedFd::from_raw_fd(vm) })
    }
}

#[cfg(target_arch = "x86_64")]
fn create_vm(kvm: BorrowedFd) -> Result<OwnedFd, KvmError> {
    let vm = unsafe { ioctl(kvm.as_raw_fd(), KVM_CREATE_VM, 0) };

    if vm < 0 {
        Err(KvmError::CreateVmFailed(Error::last_os_error()))
    } else {
        Ok(unsafe { OwnedFd::from_raw_fd(vm) })
    }
}

#[cfg(target_arch = "aarch64")]
fn load_feats(cpu: BorrowedFd) -> Result<CpuFeats, KvmError> {
    use self::ffi::{KvmOneReg, ARM64_SYS_REG, KVM_GET_ONE_REG};
    use crate::vmm::hv::{Mmfr0, Mmfr1, Mmfr2};

    // ID_AA64MMFR0_EL1.
    let mut mmfr0 = Mmfr0::default();
    let mut req = KvmOneReg {
        id: ARM64_SYS_REG(0b11, 0b000, 0b0000, 0b0111, 0b000),
        addr: &mut mmfr0,
    };

    if unsafe { ioctl(cpu.as_raw_fd(), KVM_GET_ONE_REG, &mut req) < 0 } {
        return Err(KvmError::ReadMmfr0Failed(Error::last_os_error()));
    }

    // ID_AA64MMFR1_EL1.
    let mut mmfr1 = Mmfr1::default();
    let mut req = KvmOneReg {
        id: ARM64_SYS_REG(0b11, 0b000, 0b0000, 0b0111, 0b001),
        addr: &mut mmfr1,
    };

    if unsafe { ioctl(cpu.as_raw_fd(), KVM_GET_ONE_REG, &mut req) < 0 } {
        return Err(KvmError::ReadMmfr1Failed(Error::last_os_error()));
    }

    // ID_AA64MMFR2_EL1.
    let mut mmfr2 = Mmfr2::default();
    let mut req = KvmOneReg {
        id: ARM64_SYS_REG(0b11, 0b000, 0b0000, 0b0111, 0b010),
        addr: &mut mmfr2,
    };

    if unsafe { ioctl(cpu.as_raw_fd(), KVM_GET_ONE_REG, &mut req) < 0 } {
        return Err(KvmError::ReadMmfr2Failed(Error::last_os_error()));
    }

    Ok(CpuFeats {
        mmfr0,
        mmfr1,
        mmfr2,
    })
}

#[cfg(target_arch = "x86_64")]
fn load_feats(_: BorrowedFd) -> Result<CpuFeats, KvmError> {
    Ok(CpuFeats {})
}

fn get_ext(kvm: BorrowedFd, id: c_int) -> Result<c_uint, Error> {
    let v = unsafe { ioctl(kvm.as_raw_fd(), KVM_CHECK_EXTENSION, id) };

    if v < 0 {
        Err(Error::last_os_error())
    } else {
        Ok(v.try_into().unwrap())
    }
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
    ram: Ram<KvmMapper>,
    #[allow(dead_code)] // kvm are needed by vm.
    kvm: OwnedFd,
}

impl Hypervisor for Kvm {
    type Mapper = KvmMapper;
    type Cpu<'a> = KvmCpu<'a>;
    type CpuErr = KvmCpuError;

    fn cpu_features(&self) -> &CpuFeats {
        &self.feats
    }

    fn ram(&self) -> &Ram<Self::Mapper> {
        &self.ram
    }

    fn ram_mut(&mut self) -> &mut Ram<Self::Mapper> {
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

        Ok(unsafe { KvmCpu::new(id, cpu, cx.cast(), self.vcpu_mmap_size) })
    }
}

/// Represents an error when [`Kvm`] fails to initialize.
#[derive(Debug, Error)]
pub enum KvmError {
    #[error("couldn't create a RAM")]
    CreateRamFailed(#[source] Error),

    #[error("couldn't open /dev/kvm")]
    OpenKvmFailed(#[source] Error),

    #[error("couldn't get KVM version")]
    GetKvmVersionFailed(#[source] Error),

    #[error("unexpected KVM version")]
    KvmVersionMismatched,

    #[error("couldn't get maximum number of vCPU for a VM")]
    GetMaxCpuFailed(#[source] Error),

    #[error("your OS does not support {0} vCPU on a VM")]
    MaxCpuTooLow(usize),

    #[cfg(target_arch = "aarch64")]
    #[error("your OS does not support KVM_CAP_ONE_REG")]
    NoKvmOneReg,

    #[cfg(target_arch = "x86_64")]
    #[error("your OS does not support KVM_CAP_EXT_CPUID")]
    NoKvmExtCpuid,

    #[cfg(target_arch = "x86_64")]
    #[error("couldn't get CPUID supported by KVM")]
    GetSupportedCpuidFailed(#[source] Error),

    #[error("your OS does not support KVM_CAP_SET_GUEST_DEBUG")]
    DebugNotSupported,

    #[error("couldn't get the size of vCPU mmap")]
    GetMmapSizeFailed(#[source] Error),

    #[cfg(target_arch = "aarch64")]
    #[error("your OS does not support KVM_CAP_ARM_VM_IPA_SIZE")]
    NoVmIpaSize,

    #[cfg(target_arch = "aarch64")]
    #[error("physical address supported by your CPU too small")]
    PhysicalAddressTooSmall,

    #[error("couldn't create a VM")]
    CreateVmFailed(#[source] Error),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't get preferred CPU target")]
    GetPreferredTargetFailed(#[source] Error),

    #[error("couldn't map the RAM to the VM")]
    MapRamFailed(#[source] Error),

    #[error("couldn't create vCPU #{0}")]
    CreateCpuFailed(usize, #[source] Error),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't initialize vCPU #{0}")]
    InitCpuFailed(usize, #[source] Error),

    #[cfg(target_arch = "x86_64")]
    #[error("couldn't set CPUID for vCPU #{0}")]
    SetCpuidFailed(usize, #[source] Error),

    #[error("couldn't enable debugging on vCPU #{0}")]
    EnableDebugFailed(usize, #[source] Error),

    #[cfg(all(target_arch = "aarch64"))]
    #[error("couldn't read ID_AA64MMFR0_EL1")]
    ReadMmfr0Failed(#[source] Error),

    #[cfg(all(target_arch = "aarch64"))]
    #[error("couldn't read ID_AA64MMFR1_EL1")]
    ReadMmfr1Failed(#[source] Error),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't read ID_AA64MMFR2_EL1")]
    ReadMmfr2Failed(#[source] Error),
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
