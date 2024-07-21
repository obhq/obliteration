use crate::error::RustError;
use std::ffi::{c_char, c_void};
use std::ptr::null_mut;

#[no_mangle]
pub unsafe extern "C-unwind" fn update_firmware(
    root: *const c_char,
    fw: *const c_char,
    cx: *mut c_void,
    status: unsafe extern "C-unwind" fn(*const c_char, u64, u64, *mut c_void),
) -> *mut RustError {
    null_mut()
}
