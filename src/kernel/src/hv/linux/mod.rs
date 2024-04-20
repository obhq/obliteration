use super::HypervisorError;
use libc::{open, O_RDWR};
use std::ffi::{c_int, c_void};
use std::io::Error;
use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd};

pub fn open_kvm() -> Result<OwnedFd, HypervisorError> {
    // Open KVM.
    let fd = unsafe { open(c"/dev/kvm".as_ptr(), O_RDWR) };

    if fd < 0 {
        return Err(HypervisorError::OpenKvmFailed(Error::last_os_error()));
    }

    // Check KVM version.
    let fd = unsafe { OwnedFd::from_raw_fd(fd) };
    let mut compat = false;

    match unsafe { kvm_check_version(fd.as_raw_fd(), &mut compat) } {
        0 => {
            if !compat {
                return Err(HypervisorError::KvmVersionMismatched);
            }
        }
        v => {
            return Err(HypervisorError::GetKvmVersionFailed(
                Error::from_raw_os_error(v),
            ))
        }
    }

    Ok(fd)
}

pub fn max_vcpus(kvm: BorrowedFd) -> Result<usize, Error> {
    let mut max = 0;

    match unsafe { kvm_max_vcpus(kvm.as_raw_fd(), &mut max) } {
        0 => Ok(max),
        v => Err(Error::from_raw_os_error(v)),
    }
}

pub fn create_vm(kvm: BorrowedFd) -> Result<OwnedFd, Error> {
    let mut vm = -1;

    match unsafe { kvm_create_vm(kvm.as_raw_fd(), &mut vm) } {
        0 => Ok(unsafe { OwnedFd::from_raw_fd(vm) }),
        v => Err(Error::from_raw_os_error(v)),
    }
}

pub fn set_user_memory_region(
    vm: BorrowedFd,
    slot: u32,
    addr: u64,
    len: u64,
    mem: *mut c_void,
) -> Result<(), Error> {
    match unsafe { kvm_set_user_memory_region(vm.as_raw_fd(), slot, addr, len, mem) } {
        0 => Ok(()),
        v => Err(Error::from_raw_os_error(v)),
    }
}

pub fn create_vcpu(vm: BorrowedFd, id: i32) -> Result<OwnedFd, Error> {
    let mut vcpu = -1;

    match unsafe { kvm_create_vcpu(vm.as_raw_fd(), id,&mut vcpu) } {
        0 => Ok(unsafe { OwnedFd::from_raw_fd(vcpu) }),
        v => Err(Error::from_raw_os_error(v)),
    }
}

extern "C" {
    fn kvm_check_version(kvm: c_int, compat: *mut bool) -> c_int;
    fn kvm_max_vcpus(kvm: c_int, max: *mut usize) -> c_int;
    fn kvm_create_vm(kvm: c_int, fd: *mut c_int) -> c_int;
    fn kvm_set_user_memory_region(
        vm: c_int,
        slot: u32,
        addr: u64,
        len: u64,
        mem: *mut c_void,
    ) -> c_int;
    fn kvm_create_vcpu(vm: c_int,id: c_int,  fd: *mut c_int) -> c_int;
}
