use crate::error::RustError;
use std::ffi::{c_char, c_void, CStr};
use std::ptr::null_mut;

#[no_mangle]
pub unsafe extern "C-unwind" fn update_firmware(
    root: *const c_char,
    fw: *const c_char,
    cx: *mut c_void,
    status: unsafe extern "C-unwind" fn(*const c_char, u64, u64, *mut c_void),
) -> *mut RustError {
    let _root_path = CStr::from_ptr(root);
    let _fw_path = CStr::from_ptr(fw);
    let _cx = cx;
    let _status = status;

    null_mut()
}
