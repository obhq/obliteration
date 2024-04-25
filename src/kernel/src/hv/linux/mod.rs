use self::run::KvmRun;
use super::HypervisorError;
use libc::{open, O_RDWR};
use std::ffi::{c_int, c_void};
use std::io::Error;
use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd};
use std::ptr::NonNull;
use thiserror::Error;

mod regs;
mod run;

#[derive(Debug)]
pub struct VCpus([VCpu; 8]);

impl VCpus {
    pub fn create(vm: BorrowedFd, mmap_size: usize) -> Result<Self, CreateVCpusError> {
        let vcpus = [
            VCpu::create(vm, 0, mmap_size).map_err(|e| CreateVCpusError::CreateVcpuFailed(e, 0))?,
            VCpu::create(vm, 1, mmap_size).map_err(|e| CreateVCpusError::CreateVcpuFailed(e, 1))?,
            VCpu::create(vm, 2, mmap_size).map_err(|e| CreateVCpusError::CreateVcpuFailed(e, 2))?,
            VCpu::create(vm, 3, mmap_size).map_err(|e| CreateVCpusError::CreateVcpuFailed(e, 3))?,
            VCpu::create(vm, 4, mmap_size).map_err(|e| CreateVCpusError::CreateVcpuFailed(e, 4))?,
            VCpu::create(vm, 5, mmap_size).map_err(|e| CreateVCpusError::CreateVcpuFailed(e, 5))?,
            VCpu::create(vm, 6, mmap_size).map_err(|e| CreateVCpusError::CreateVcpuFailed(e, 6))?,
            VCpu::create(vm, 7, mmap_size).map_err(|e| CreateVCpusError::CreateVcpuFailed(e, 7))?,
        ];

        Ok(Self (vcpus))
    }
}

#[derive(Debug)]
struct VCpu {
    fd: OwnedFd,
    kvm_run: NonNull<KvmRun>,
}

impl VCpu {
    pub fn create(vm: BorrowedFd, id: i32, mmap_size: usize) -> Result<Self, CreateVCpuError> {
        use libc::{MAP_SHARED, PROT_READ, PROT_WRITE};

        let fd = create_vcpu(vm, id).map_err(CreateVCpuError::CreateVcpuFailed)?;

        let kvm_run = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                mmap_size,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                fd.as_raw_fd(),
                0,
            )
        };

        if kvm_run == libc::MAP_FAILED {
            return Err(CreateVCpuError::MmapFailed(Error::last_os_error()));
        }

        Ok(Self {
            fd,
            kvm_run: NonNull::new(kvm_run.cast()).unwrap(),
        })
    }
}

#[derive(Debug, Error)]
pub enum CreateVCpusError {
    #[error("Failed to create vCPU #{1}")]
    CreateVcpuFailed(#[source] CreateVCpuError, u8),
}

#[derive(Debug, Error)]
pub enum CreateVCpuError {
    #[error("Failed to create vCPU")]
    CreateVcpuFailed(#[source] Error),

    #[error("Failed to mmap KVM_RUN")]
    MmapFailed(#[source] Error),
}

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

pub fn get_vcpu_mmap_size(kvm: BorrowedFd) -> Result<usize, Error> {
    match unsafe { kvm_get_vcpu_mmap_size(kvm.as_raw_fd()) } {
        size @ 0.. => Ok(size as usize),
        v => Err(Error::from_raw_os_error(v)),
    }
}

pub fn create_vcpu(vm: BorrowedFd, id: i32) -> Result<OwnedFd, Error> {
    let mut vcpu = -1;

    match unsafe { kvm_create_vcpu(vm.as_raw_fd(), id, &mut vcpu) } {
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
    fn kvm_get_vcpu_mmap_size(kvm: c_int) -> c_int;
    fn kvm_create_vcpu(vm: c_int, id: c_int, fd: *mut c_int) -> c_int;
    fn kvm_run(vcpu: c_int) -> c_int;
}
