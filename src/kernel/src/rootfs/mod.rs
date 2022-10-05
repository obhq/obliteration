use crate::fs::{Directory, Fs, MountPoints};

mod directories;

#[no_mangle]
pub extern "C" fn kernel_rootfs_new(_: *mut *mut error::Error) -> *mut RootFs<'static> {
    let fs = Box::new(RootFs {
        mounts: MountPoints::new(),
    });

    Box::into_raw(fs)
}

#[no_mangle]
pub extern "C" fn kernel_rootfs_free(fs: *mut RootFs<'static>) {
    unsafe { Box::from_raw(fs) };
}

/// Represents a virtual root file system. The directory structure the kernel will see will be the
/// same as PS4 while the actual structure in the host will be different.
#[derive(Debug)]
pub struct RootFs<'fs> {
    mounts: MountPoints<'fs>,
}

impl<'fs> Fs<'fs> for RootFs<'fs> {
    fn root(&'fs self) -> Box<dyn Directory<'fs> + 'fs> {
        Box::new(directories::Root::new(self))
    }
}
