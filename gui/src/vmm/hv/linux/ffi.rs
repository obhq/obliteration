use std::ffi::{c_int, c_ulong};

pub const KVM_GET_API_VERSION: c_ulong = _IO(KVMIO, 0x00);
pub const KVM_CREATE_VM: c_ulong = _IO(KVMIO, 0x01);
pub const KVM_CHECK_EXTENSION: c_ulong = _IO(KVMIO, 0x03);
pub const KVM_GET_VCPU_MMAP_SIZE: c_ulong = _IO(KVMIO, 0x04);
pub const KVM_CREATE_VCPU: c_ulong = _IO(KVMIO, 0x41);
pub const KVM_SET_USER_MEMORY_REGION: c_ulong = _IOW::<KvmUserspaceMemoryRegion>(KVMIO, 0x46);
#[cfg(target_arch = "aarch64")]
pub const KVM_GET_ONE_REG: c_ulong = _IOW::<KvmOneReg<()>>(KVMIO, 0xab);
#[cfg(target_arch = "aarch64")]
pub const KVM_SET_ONE_REG: c_ulong = _IOW::<KvmOneReg<()>>(KVMIO, 0xac);
#[cfg(target_arch = "aarch64")]
pub const KVM_ARM_VCPU_INIT: c_ulong = _IOW::<KvmVcpuInit>(KVMIO, 0xae);
#[cfg(target_arch = "aarch64")]
pub const KVM_ARM_PREFERRED_TARGET: c_ulong = _IOR::<KvmVcpuInit>(KVMIO, 0xaf);

pub const KVM_API_VERSION: c_int = 12;

pub const KVM_CAP_SET_GUEST_DEBUG: c_int = 23;
pub const KVM_CAP_MAX_VCPUS: c_int = 66;
#[cfg(target_arch = "aarch64")]
pub const KVM_CAP_ONE_REG: c_int = 70;
#[cfg(target_arch = "aarch64")]
pub const KVM_CAP_ARM_VM_IPA_SIZE: c_int = 165;

const KVMIO: c_ulong = 0xAE;

const _IOC_NONE: c_ulong = 0;
const _IOC_WRITE: c_ulong = 1;
const _IOC_READ: c_ulong = 2;

const _IOC_NRSHIFT: c_ulong = 0;
const _IOC_NRBITS: c_ulong = 8;
const _IOC_TYPEBITS: c_ulong = 8;
const _IOC_SIZEBITS: c_ulong = 14;
const _IOC_TYPESHIFT: c_ulong = _IOC_NRSHIFT + _IOC_NRBITS;
const _IOC_SIZESHIFT: c_ulong = _IOC_TYPESHIFT + _IOC_TYPEBITS;
const _IOC_DIRSHIFT: c_ulong = _IOC_SIZESHIFT + _IOC_SIZEBITS;

#[cfg(target_arch = "aarch64")]
#[allow(non_snake_case)]
pub fn KVM_VM_TYPE_ARM_IPA_SIZE(v: c_int) -> c_int {
    v & 0xff
}

#[cfg(target_arch = "aarch64")]
#[allow(non_snake_case)]
pub fn ARM64_SYS_REG(op0: u64, op1: u64, crn: u64, crm: u64, op2: u64) -> u64 {
    (0x6000000000000000
        | 0x0013 << 16
        | (op0 << 14) & 0x000000000000c000
        | (op1 << 11) & 0x0000000000003800
        | (crn << 7) & 0x0000000000000780
        | (crm << 3) & 0x0000000000000078
        | op2 & 0x0000000000000007)
        | 0x0030000000000000
}

#[allow(non_snake_case)]
const fn _IO(ty: c_ulong, nr: c_ulong) -> c_ulong {
    _IOC(_IOC_NONE, ty, nr, 0)
}

#[allow(non_snake_case)]
const fn _IOR<T>(ty: c_ulong, nr: c_ulong) -> c_ulong {
    _IOC(_IOC_READ, ty, nr, size_of::<T>() as _)
}

#[allow(non_snake_case)]
const fn _IOW<T>(ty: c_ulong, nr: c_ulong) -> c_ulong {
    _IOC(_IOC_WRITE, ty, nr, size_of::<T>() as _)
}

#[allow(non_snake_case)]
const fn _IOC(dir: c_ulong, ty: c_ulong, nr: c_ulong, size: c_ulong) -> c_ulong {
    (dir << _IOC_DIRSHIFT)
        | (ty << _IOC_TYPESHIFT)
        | (nr << _IOC_NRSHIFT)
        | (size << _IOC_SIZESHIFT)
}

#[repr(C)]
pub struct KvmUserspaceMemoryRegion {
    pub slot: u32,
    pub flags: u32,
    pub guest_phys_addr: u64,
    pub memory_size: u64,
    pub userspace_addr: u64,
}

#[cfg(target_arch = "aarch64")]
#[repr(C)]
pub struct KvmOneReg<'a, T> {
    pub id: u64,
    pub addr: &'a mut T,
}

#[cfg(target_arch = "aarch64")]
#[repr(C)]
pub struct KvmVcpuInit {
    pub target: u32,
    pub features: [u32; 7],
}

extern "C" {
    pub fn kvm_run(vcpu: c_int) -> c_int;
}
