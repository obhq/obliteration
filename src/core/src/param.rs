use crate::ffi::QString;
use error::Error;
use param::Param;
use std::ffi::{c_char, CStr};
use std::fs::File;
use std::ptr::null_mut;

#[no_mangle]
pub unsafe extern "C" fn param_open(file: *const c_char, error: *mut *mut Error) -> *mut Param {
    // Open file.
    let file = match File::open(CStr::from_ptr(file).to_str().unwrap()) {
        Ok(v) => v,
        Err(e) => {
            *error = Error::new(e);
            return null_mut();
        }
    };

    // Parse.
    let param = match Param::read(file) {
        Ok(v) => v,
        Err(e) => {
            *error = Error::new(e);
            return null_mut();
        }
    };

    Box::into_raw(param.into())
}

#[no_mangle]
pub unsafe extern "C" fn param_close(param: *mut Param) {
    drop(Box::from_raw(param));
}

#[no_mangle]
pub unsafe extern "C" fn param_app_ver_get(param: &Param, buf: &mut QString) {
    buf.set(param.app_ver().unwrap_or(""));
}

#[no_mangle]
pub unsafe extern "C" fn param_category_get(param: &Param, buf: &mut QString) {
    buf.set(param.category());
}

#[no_mangle]
pub unsafe extern "C" fn param_content_id_get(param: &Param, buf: &mut QString) {
    buf.set(param.content_id());
}

#[no_mangle]
pub unsafe extern "C" fn param_short_content_id_get(param: &Param, buf: &mut QString) {
    buf.set(param.shortcontent_id());
}

#[no_mangle]
pub unsafe extern "C" fn param_title_get(param: &Param, buf: &mut QString) {
    buf.set(param.title().unwrap_or(""));
}

#[no_mangle]
pub unsafe extern "C" fn param_title_id_get(param: &Param, buf: &mut QString) {
    buf.set(param.title_id())
}

#[no_mangle]
pub unsafe extern "C" fn param_version_get(param: &Param, buf: &mut QString) {
    match param.version() {
        Some(version) => buf.set(version),
        None => buf.set(""),
    }
}
