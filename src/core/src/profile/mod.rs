use crate::error::RustError;
use crate::string::strdup;
use serde::{Deserialize, Serialize};
use std::ffi::{c_char, CStr, CString};
use std::fs::File;
use std::path::Path;
use std::ptr::null_mut;
use std::time::SystemTime;
use uuid::Uuid;

#[no_mangle]
pub unsafe extern "C" fn profile_new(name: *const c_char) -> *mut Profile {
    Box::into_raw(Box::new(Profile {
        id: Uuid::new_v4(),
        name: CStr::from_ptr(name).to_owned(),
        created: SystemTime::now(),
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
            *err = RustError::new("the specified path is not UTF-8");
            return null_mut();
        }
    };

    // TODO: Use from_io() once https://github.com/jamesmunns/postcard/issues/162 is implemented.
    let path = root.join("profile.bin");
    let data = match std::fs::read(&path) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source(format_args!("couldn't read {}", path.display()), e);
            return null_mut();
        }
    };

    // Load profile.bin.
    let p = match postcard::from_bytes(&data) {
        Ok(v) => v,
        Err(e) => {
            *err = RustError::with_source(format_args!("couldn't load {}", path.display()), e);
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
pub unsafe extern "C" fn profile_save(p: *const Profile, path: *const c_char) -> *mut RustError {
    // Check if path UTF-8.
    let root = match CStr::from_ptr(path).to_str() {
        Ok(v) => Path::new(v),
        Err(_) => return RustError::new("the specified path is not UTF-8"),
    };

    // Create a directory.
    if let Err(e) = std::fs::create_dir_all(root) {
        return RustError::with_source("couldn't create the specified path", e);
    }

    // Create profile.bin.
    let path = root.join("profile.bin");
    let file = match File::create(&path) {
        Ok(v) => v,
        Err(e) => {
            return RustError::with_source(format_args!("couldn't create {}", path.display()), e)
        }
    };

    // Write profile.bin.
    if let Err(e) = postcard::to_io(&*p, file) {
        return RustError::with_source(format_args!("couldn't write {}", path.display()), e);
    }

    null_mut()
}

/// Contains settings to launch the kernel.
#[derive(Deserialize, Serialize)]
pub struct Profile {
    id: Uuid,
    name: CString,
    created: SystemTime,
}
