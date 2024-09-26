use std::ffi::{c_int, c_ulong, c_void};

pub const KVM_GET_API_VERSION: c_ulong = _IO(KVMIO, 0x00);
pub const KVM_CREATE_VM: c_ulong = _IO(KVMIO, 0x01);
pub const KVM_CHECK_EXTENSION: c_ulong = _IO(KVMIO, 0x03);
pub const KVM_GET_VCPU_MMAP_SIZE: c_ulong = _IO(KVMIO, 0x04);

pub const KVM_API_VERSION: c_int = 12;

pub const KVM_CAP_MAX_VCPUS: c_int = 66;
pub const KVM_CAP_ONE_REG: c_int = 70;
pub const KVM_CAP_ARM_VM_IPA_SIZE: c_int = 165;

const KVMIO: c_ulong = 0xAE;
const _IOC_NONE: c_ulong = 0;
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

#[allow(non_snake_case)]
const fn _IO(ty: c_ulong, nr: c_ulong) -> c_ulong {
    _IOC(_IOC_NONE, ty, nr, 0)
}

#[allow(non_snake_case)]
const fn _IOC(dir: c_ulong, ty: c_ulong, nr: c_ulong, size: c_ulong) -> c_ulong {
    ((dir) << _IOC_DIRSHIFT)
        | ((ty) << _IOC_TYPESHIFT)
        | ((nr) << _IOC_NRSHIFT)
        | ((size) << _IOC_SIZESHIFT)
}

extern "C" {
    pub fn kvm_set_user_memory_region(
        vm: c_int,
        slot: u32,
        addr: u64,
        len: u64,
        mem: *mut c_void,
    ) -> c_int;
    pub fn kvm_create_vcpu(vm: c_int, id: u32, fd: *mut c_int) -> c_int;

    pub fn kvm_run(vcpu: c_int) -> c_int;
}
