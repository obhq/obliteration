use crate::NewError;
use libc::{open, O_RDWR};
use std::io::Error;
use std::os::fd::{FromRawFd, OwnedFd};

pub fn kvm_new() -> Result<OwnedFd, NewError> {
    // Open KVM.
    let fd = unsafe { open(b"/dev/kvm\0".as_ptr().cast(), O_RDWR) };

    if fd < 0 {
        return Err(NewError::OpenKvmFailed("/dev/kvm", Error::last_os_error()));
    }

    let fd = unsafe { OwnedFd::from_raw_fd(fd) };

    Ok(fd)
}
