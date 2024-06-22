use std::ffi::{c_int, c_void};

extern "C" {
    pub fn hv_vm_create(config: *mut ()) -> c_int;
    pub fn hv_vm_destroy() -> c_int;
    pub fn hv_capability(capability: u64, value: *mut u64) -> c_int;
    pub fn hv_vm_map(uva: *mut c_void, gpa: u64, size: usize, flags: u64) -> c_int;
    pub fn hv_vcpu_create(vcpu: *mut u64, flags: u64) -> c_int;
    pub fn hv_vcpu_destroy(vcpu: u64) -> c_int;
}
