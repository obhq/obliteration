use super::RustError;
use std::ffi::c_char;

#[no_mangle]
pub unsafe extern "C" fn error_free(e: *mut RustError) {
    drop(Box::from_raw(e));
}

#[no_mangle]
pub unsafe extern "C" fn error_message(e: *const RustError) -> *const c_char {
    (*e).0.as_ptr()
}
