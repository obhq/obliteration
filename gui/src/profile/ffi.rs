use super::{DisplayResolution, Profile};
use crate::error::RustError;
use crate::string::strdup;
use std::ffi::{c_char, CStr};
use std::fs::File;
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

    // Open profile.bin.
    let path = root.join("profile.bin");
    let file = match File::open(&path) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source(format_args!("couldn't open {}", path.display()), e)
                .into_c();
            return null_mut();
        }
    };

    // Load profile.bin.
    let p = match ciborium::from_reader(file) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source(format_args!("couldn't load {}", path.display()), e)
                .into_c();
            return null_mut();
        }
    };

    Box::into_raw(Box::new(p))
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

    // Create a directory.
    if let Err(e) = std::fs::create_dir_all(root) {
        return RustError::with_source("couldn't create the specified path", e).into_c();
    }

    // Create profile.bin.
    let path = root.join("profile.bin");
    let file = match File::create(&path) {
        Ok(v) => v,
        Err(e) => {
            return RustError::with_source(format_args!("couldn't create {}", path.display()), e)
                .into_c()
        }
    };

    // Write profile.bin.
    if let Err(e) = ciborium::into_writer(&*p, file) {
        return RustError::with_source(format_args!("couldn't write {}", path.display()), e)
            .into_c();
    }

    null_mut()
}
