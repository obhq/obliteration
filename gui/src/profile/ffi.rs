use super::{DisplayResolution, Profile};
use crate::error::RustError;
use crate::string::strdup;
use std::ffi::{c_char, CStr};
use std::path::Path;
use std::ptr::null_mut;

#[no_mangle]
pub unsafe extern "C" fn profile_new(name: *const c_char) -> *mut Profile {
    Box::into_raw(Box::new(Profile {
        name: CStr::from_ptr(name).to_owned(),
        ..Default::default()
    }))
}

#[no_mangle]
pub unsafe extern "C" fn profile_load(
    path: *const c_char,
    err: *mut *mut RustError,
) -> *mut Profile {
    // Check if path UTF-8.
    let root = match CStr::from_ptr(path).to_str() {
        Ok(v) => Path::new(v),
        Err(_) => {
            *err = RustError::new("the specified path is not UTF-8").into_c();
            return null_mut();
        }
    };

    match Profile::load(root) {
        Ok(v) => Box::into_raw(Box::new(v)),
        Err(e) => {
            *err = RustError::wrap(e).into_c();

            null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn profile_free(p: *mut Profile) {
    drop(Box::from_raw(p));
}

#[no_mangle]
pub unsafe extern "C" fn profile_id(p: *const Profile) -> *mut c_char {
    strdup((*p).id.to_string())
}

#[no_mangle]
pub unsafe extern "C" fn profile_name(p: *const Profile) -> *const c_char {
    (*p).name.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn profile_display_resolution(p: *const Profile) -> DisplayResolution {
    (*p).display_resolution
}

#[no_mangle]
pub unsafe extern "C" fn profile_set_display_resolution(p: *mut Profile, v: DisplayResolution) {
    (*p).display_resolution = v;
}

#[no_mangle]
pub unsafe extern "C" fn profile_save(p: *const Profile, path: *const c_char) -> *mut RustError {
    // Check if path UTF-8.
    let root = match CStr::from_ptr(path).to_str() {
        Ok(v) => Path::new(v),
        Err(_) => return RustError::new("the specified path is not UTF-8").into_c(),
    };

    match (*p).save(root) {
        Ok(_) => null_mut(),
        Err(e) => RustError::wrap(e).into_c(),
    }
}
