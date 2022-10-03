use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

pub type Logger = extern "C" fn(c_int, c_int, *const c_char, ud: *mut c_void);

#[no_mangle]
pub extern "C" fn kernel_new(_: *mut *mut error::Error) -> *mut Kernel {
    let krn = Box::new(Kernel { logger: None });

    Box::into_raw(krn)
}

#[no_mangle]
pub extern "C" fn kernel_shutdown(krn: *mut Kernel) {
    unsafe { Box::from_raw(krn) };
}

#[no_mangle]
pub extern "C" fn kernel_set_logger(krn: &mut Kernel, logger: Option<Logger>, ud: *mut c_void) {
    krn.logger = match logger {
        Some(v) => Some((v, ud)),
        None => None,
    };
}

pub struct Kernel {
    logger: Option<(Logger, *mut c_void)>,
}
