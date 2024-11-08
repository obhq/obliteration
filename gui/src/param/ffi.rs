use crate::error::RustError;
use crate::string::strdup;
use param::Param;
use std::ffi::{c_char, CStr};
use std::ptr::null_mut;

#[no_mangle]
pub unsafe extern "C" fn param_open(file: *const c_char, error: *mut *mut RustError) -> *mut Param {
    let path = CStr::from_ptr(file).to_str().unwrap();

    match Param::open(path) {
        Ok(param) => Box::into_raw(Box::new(param)),
        Err(e) => {
            *error = RustError::wrap(e).into_c();
            null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn param_close(p: *mut Param) {
    drop(Box::from_raw(p));
}

#[no_mangle]
pub unsafe extern "C" fn param_app_ver_get(p: *const Param) -> *mut c_char {
    (*p).app_ver().map(strdup).unwrap_or(null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn param_category_get(p: *const Param) -> *mut c_char {
    strdup((*p).category())
}

#[no_mangle]
pub unsafe extern "C" fn param_content_id_get(p: *const Param) -> *mut c_char {
    strdup((*p).content_id())
}

#[no_mangle]
pub unsafe extern "C" fn param_short_content_id_get(p: *const Param) -> *mut c_char {
    strdup((*p).shortcontent_id())
}

#[no_mangle]
pub unsafe extern "C" fn param_title_get(p: *const Param) -> *mut c_char {
    (*p).title().map(strdup).unwrap_or(null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn param_title_id_get(p: *const Param) -> *mut c_char {
    strdup((*p).title_id())
}

#[no_mangle]
pub unsafe extern "C" fn param_version_get(p: *const Param) -> *mut c_char {
    (*p).version().map(strdup).unwrap_or(null_mut())
}
