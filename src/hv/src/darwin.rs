#![allow(non_camel_case_types)]

use std::ffi::c_int;

#[repr(C)]
pub struct hv_vm_config_t([u8; 0]);

extern "C" {
    pub fn hv_vm_create(config: *mut hv_vm_config_t) -> c_int;
    pub fn hv_vm_destroy() -> c_int;
}
