use super::regs::KvmRegs;
use std::ffi::{c_int, c_void};

extern "C" {
    pub fn kvm_check_version(kvm: c_int, compat: *mut bool) -> c_int;
    pub fn kvm_max_vcpus(kvm: c_int, max: *mut usize) -> c_int;
    pub fn kvm_create_vm(kvm: c_int, fd: *mut c_int) -> c_int;
    pub fn kvm_get_vcpu_mmap_size(kvm: c_int) -> c_int;

    pub fn kvm_set_user_memory_region(
        vm: c_int,
        slot: u32,
        addr: u64,
        len: u64,
        mem: *mut c_void,
    ) -> c_int;
    pub fn kvm_create_vcpu(vm: c_int, id: u32, fd: *mut c_int) -> c_int;

    pub fn kvm_run(vcpu: c_int) -> c_int;
    pub fn kvm_get_regs(vcpu: c_int, regs: *mut KvmRegs) -> c_int;
    pub fn kvm_set_regs(vcpu: c_int, regs: *const KvmRegs) -> c_int;
}
