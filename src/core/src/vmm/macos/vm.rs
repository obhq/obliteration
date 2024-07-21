use super::ffi::{hv_capability, hv_vm_create, hv_vm_destroy, hv_vm_map};
use std::ffi::{c_int, c_void};
use std::num::NonZero;
use std::ptr::null_mut;

/// RAII struct for `hv_vm_create` and `hv_vm_destroy`.
pub struct Vm(());

impl Vm {
    pub fn new() -> Result<Self, NonZero<c_int>> {
        let ret = unsafe { hv_vm_create(null_mut()) };

        match NonZero::new(ret) {
            Some(ret) => Err(ret),
            None => Ok(Self(())),
        }
    }

    pub fn capability(&self, id: u64) -> Result<u64, NonZero<c_int>> {
        let mut value = 0;
        let ret = unsafe { hv_capability(id, &mut value) };

        match NonZero::new(ret) {
            Some(ret) => Err(ret),
            None => Ok(value),
        }
    }

    pub fn vm_map(&self, host: *mut c_void, guest: u64, len: usize) -> Result<(), NonZero<c_int>> {
        let ret = unsafe { hv_vm_map(host, guest, len, 1 | 2 | 4) };

        match NonZero::new(ret) {
            Some(ret) => Err(ret),
            None => Ok(()),
        }
    }
}

impl Drop for Vm {
    fn drop(&mut self) {
        let status = unsafe { hv_vm_destroy() };

        if status != 0 {
            panic!("hv_vm_destroy() fails with {status:#x}");
        }
    }
}
