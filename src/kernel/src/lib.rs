#[no_mangle]
pub extern "C" fn kernel_new(_: *mut *mut error::Error) -> *mut Kernel {
    let krn = Box::new(Kernel {});

    Box::into_raw(krn)
}

#[no_mangle]
pub extern "C" fn kernel_shutdown(krn: *mut Kernel) {
    unsafe { Box::from_raw(krn) };
}

pub struct Kernel {}
