use error::Error;
use param::Param;
use pkg::Pkg;
use std::ffi::{c_char, c_void, CStr};
use std::ptr::null_mut;

#[no_mangle]
pub unsafe extern "C" fn pkg_open(file: *const c_char, error: *mut *mut Error) -> *mut Pkg {
    let path = CStr::from_ptr(file);
    let pkg = match Pkg::open(path.to_str().unwrap()) {
        Ok(v) => Box::new(v),
        Err(e) => {
            *error = Error::new(e);
            return null_mut();
        }
    };

    Box::into_raw(pkg)
}

#[no_mangle]
pub unsafe extern "C" fn pkg_close(pkg: *mut Pkg) {
    drop(Box::from_raw(pkg));
}

#[no_mangle]
pub unsafe extern "C" fn pkg_get_param(pkg: &Pkg, error: *mut *mut Error) -> *mut Param {
    let param = match pkg.get_param() {
        Ok(v) => Box::new(v),
        Err(e) => {
            *error = Error::new(e);
            return null_mut();
        }
    };

    Box::into_raw(param)
}

#[no_mangle]
pub unsafe extern "C" fn pkg_extract(
    pkg: &Pkg,
    dir: *const c_char,
    status: extern "C" fn(*const c_char, usize, usize, *mut c_void),
    ud: *mut c_void,
) -> *mut Error {
    let dir = CStr::from_ptr(dir);

    match pkg.extract(dir.to_str().unwrap(), status, ud) {
        Ok(_) => null_mut(),
        Err(e) => Error::new(e),
    }
}
