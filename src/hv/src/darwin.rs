use crate::NewError;
use std::ffi::c_int;
use std::ptr::null_mut;

/// RAII struct for `hv_vm_create` and `hv_vm_destroy`.
pub struct Vm(());

impl Vm {
    pub fn new() -> Result<Self, NewError> {
        match unsafe { hv_vm_create(null_mut()) } {
            0 => Ok(Self(())),
            v => Err(NewError::CreateVmFailed(v)),
        }
    }

    pub fn capability(&self, id: u64) -> Result<u64, c_int> {
        let mut value = 0;

        match unsafe { hv_capability(id, &mut value) } {
            0 => Ok(value),
            v => Err(v),
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

extern "C" {
    fn hv_vm_create(config: *mut ()) -> c_int;
    fn hv_vm_destroy() -> c_int;
    fn hv_capability(capability: u64, value: *mut u64) -> c_int;
}
