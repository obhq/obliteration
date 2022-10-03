use crate::Fs;

#[no_mangle]
pub extern "C" fn kernel_pfs_new(_: *mut *mut error::Error) -> *mut Pfs {
    let pfs = Box::new(Pfs {});

    Box::into_raw(pfs)
}

pub struct Pfs {}

impl Fs for Pfs {}
