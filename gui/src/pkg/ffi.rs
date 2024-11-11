use super::ExtractProgress;
use crate::error::RustError;
use param::Param;
use pkg::Pkg;
use std::ffi::{c_char, c_void, CStr};
use std::path::Path;
use std::ptr::null_mut;

#[no_mangle]
pub unsafe extern "C" fn pkg_open(file: *const c_char, error: *mut *mut RustError) -> *mut Pkg {
    let path = CStr::from_ptr(file).to_str().unwrap();

    match Pkg::open(path) {
        Ok(pkg) => Box::into_raw(Box::new(pkg)),
        Err(e) => {
            *error = RustError::wrap(e).into_c();
            null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn pkg_close(pkg: *mut Pkg) {
    drop(Box::from_raw(pkg));
}

#[no_mangle]
pub unsafe extern "C" fn pkg_get_param(pkg: *const Pkg, error: *mut *mut RustError) -> *mut Param {
    match (*pkg).get_param() {
        Ok(param) => Box::into_raw(Box::new(param)),
        Err(e) => {
            *error = RustError::wrap(e).into_c();
            null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn pkg_extract(
    pkg: *const Pkg,
    dir: *const c_char,
    status: extern "C" fn(*const c_char, usize, u64, u64, *mut c_void),
    ud: *mut c_void,
) -> *mut RustError {
    let root: &Path = CStr::from_ptr(dir).to_str().unwrap().as_ref();
    let progress = ExtractProgress {
        status,
        ud,
        root,
        total: 0,
        progress: 0,
    };

    match (*pkg).extract(root, progress) {
        Ok(_) => null_mut(),
        Err(e) => RustError::wrap(e).into_c(),
    }
}
