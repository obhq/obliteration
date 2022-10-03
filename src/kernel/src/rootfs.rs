use crate::Fs;

#[no_mangle]
pub extern "C" fn kernel_rootfs_new(_: *mut *mut error::Error) -> *mut RootFs {
    let fs = Box::new(RootFs {});

    Box::into_raw(fs)
}

#[no_mangle]
pub extern "C" fn kernel_rootfs_free(fs: *mut RootFs) {
    unsafe { Box::from_raw(fs) };
}

/// Represents a virtual root file system. The directory structure that kernel will see will be the
/// same as PS4 while the actual structure in the host will be different.
pub struct RootFs {}

impl Fs for RootFs {}
