use crate::NewError;
use libc::{open, O_RDWR};
use std::ffi::c_int;
use std::io::Error;
use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd};

pub fn open_kvm() -> Result<OwnedFd, NewError> {
    // Open KVM.
    let fd = unsafe { open(b"/dev/kvm\0".as_ptr().cast(), O_RDWR) };

    if fd < 0 {
        return Err(NewError::OpenKvmFailed("/dev/kvm", Error::last_os_error()));
    }

    // Check KVM version.
    let fd = unsafe { OwnedFd::from_raw_fd(fd) };
    let mut compat = false;

    match unsafe { kvm_check_version(fd.as_raw_fd(), &mut compat) } {
        0 => {
            if !compat {
                return Err(NewError::KvmVersionMismatched);
            }
        }
        v => return Err(NewError::GetKvmVersionFailed(Error::from_raw_os_error(v))),
    }

    Ok(fd)
}

pub fn create_vm(kvm: BorrowedFd) -> Result<OwnedFd, NewError> {
    let mut vm = -1;

    match unsafe { kvm_create_vm(kvm.as_raw_fd(), &mut vm) } {
        0 => Ok(unsafe { OwnedFd::from_raw_fd(vm) }),
        v => Err(NewError::CreateVmFailed(Error::from_raw_os_error(v))),
    }
}

extern "C" {
    fn kvm_check_version(kvm: c_int, compat: *mut bool) -> c_int;
    fn kvm_create_vm(kvm: c_int, fd: *mut c_int) -> c_int;
}
