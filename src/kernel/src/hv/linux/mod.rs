use self::regs::KvmRegs;
use self::run::KvmRun;
use super::HypervisorError;
use std::ffi::{c_int, c_void};
use std::io::Error;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::ptr::NonNull;
use thiserror::Error;

mod regs;
mod run;

pub struct Kvm(OwnedFd);

impl Kvm {
    pub fn open() -> Result<Self, HypervisorError> {
        use libc::{open, O_RDWR};

        let fd = unsafe { open(c"/dev/kvm".as_ptr(), O_RDWR) };

        if fd < 0 {
            return Err(HypervisorError::OpenKvmFailed(Error::last_os_error()));
        }

        // Check KVM version.
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        let mut compat = false;

        match unsafe { kvm_check_version(fd.as_raw_fd(), &mut compat) } {
            0 if !compat => {
                return Err(HypervisorError::KvmVersionMismatched);
            }
            0 => {}
            v => {
                return Err(HypervisorError::GetKvmVersionFailed(
                    Error::from_raw_os_error(v),
                ))
            }
        }

        Ok(Self(fd))
    }

    pub fn get_vcpu_mmap_size(&self) -> Result<usize, HypervisorError> {
        match unsafe { kvm_get_vcpu_mmap_size(self.0.as_raw_fd()) } {
            size @ 0.. => Ok(size as usize),
            _ => Err(HypervisorError::GetMmapSizeFailed(Error::last_os_error())),
        }
    }

    pub fn max_vcpus(&self) -> Result<usize, HypervisorError> {
        let mut max = 0;

        match unsafe { kvm_max_vcpus(self.0.as_raw_fd(), &mut max) } {
            0 => Ok(max),
            v => Err(HypervisorError::GetMaxCpuFailed(Error::from_raw_os_error(
                v,
            ))),
        }
    }

    pub fn create_vm(&self) -> Result<Vm, HypervisorError> {
        let mut vm = -1;

        match unsafe { kvm_create_vm(self.0.as_raw_fd(), &mut vm) } {
            0 => Ok(Vm(unsafe { OwnedFd::from_raw_fd(vm) })),
            v => Err(HypervisorError::CreateVmFailed(Error::from_raw_os_error(v))),
        }
    }
}

pub struct Vm(OwnedFd);

impl Vm {
    pub fn set_user_memory_region(
        &self,
        slot: u32,
        addr: u64,
        len: u64,
        mem: *mut c_void,
    ) -> Result<(), HypervisorError> {
        match unsafe { kvm_set_user_memory_region(self.0.as_raw_fd(), slot, addr, len, mem) } {
            0 => Ok(()),
            v => Err(HypervisorError::MapRamFailed(Error::from_raw_os_error(v))),
        }
    }

    pub fn create_vcpus(&self, mmap_size: usize) -> Result<VCpus, CreateVCpusError> {
        let vcpus = [
            self.create_vcpu(0, mmap_size)
                .map_err(|e| CreateVCpusError::CreateVcpuFailed(e, 0))?,
            self.create_vcpu(1, mmap_size)
                .map_err(|e| CreateVCpusError::CreateVcpuFailed(e, 1))?,
            self.create_vcpu(2, mmap_size)
                .map_err(|e| CreateVCpusError::CreateVcpuFailed(e, 2))?,
            self.create_vcpu(3, mmap_size)
                .map_err(|e| CreateVCpusError::CreateVcpuFailed(e, 3))?,
            self.create_vcpu(4, mmap_size)
                .map_err(|e| CreateVCpusError::CreateVcpuFailed(e, 4))?,
            self.create_vcpu(5, mmap_size)
                .map_err(|e| CreateVCpusError::CreateVcpuFailed(e, 5))?,
            self.create_vcpu(6, mmap_size)
                .map_err(|e| CreateVCpusError::CreateVcpuFailed(e, 6))?,
            self.create_vcpu(7, mmap_size)
                .map_err(|e| CreateVCpusError::CreateVcpuFailed(e, 7))?,
        ];

        Ok(VCpus(vcpus))
    }

    fn create_vcpu(&self, id: i32, mmap_size: usize) -> Result<VCpu, CreateVCpuError> {
        use libc::{mmap, MAP_FAILED, MAP_SHARED, PROT_READ, PROT_WRITE};

        let mut vcpu = -1;

        let fd = match unsafe { kvm_create_vcpu(self.0.as_raw_fd(), id, &mut vcpu) } {
            0 => Ok(unsafe { OwnedFd::from_raw_fd(vcpu) }),
            v => Err(CreateVCpuError::CreateVcpuFailed(Error::from_raw_os_error(
                v,
            ))),
        }?;

        let kvm_run = unsafe {
            mmap(
                std::ptr::null_mut(),
                mmap_size,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                fd.as_raw_fd(),
                0,
            )
        };

        if kvm_run == MAP_FAILED {
            return Err(CreateVCpuError::MmapFailed(Error::last_os_error()));
        }

        Ok(VCpu {
            fd,
            kvm_run: NonNull::new(kvm_run.cast()).unwrap(),
            mmap_size,
        })
    }
}

#[derive(Debug)]
pub struct VCpus([VCpu; 8]);

#[derive(Debug)]
struct VCpu {
    fd: OwnedFd,
    kvm_run: NonNull<KvmRun>,
    mmap_size: usize,
}

impl Drop for VCpu {
    fn drop(&mut self) {
        use libc::munmap;

        unsafe {
            if munmap(self.kvm_run.as_ptr().cast(), self.mmap_size) < 0 {
                panic!("failed to munmap KVM_RUN: {}", Error::last_os_error());
            };
        }
    }
}

impl VCpu {
    pub fn get_regs(&self) -> Result<KvmRegs, Error> {
        let mut regs = MaybeUninit::uninit();

        match unsafe { kvm_get_regs(self.fd.as_raw_fd(), regs.as_mut_ptr()) } {
            0 => Ok(unsafe { regs.assume_init() }),
            _ => Err(Error::last_os_error()),
        }
    }

    pub fn set_regs(&self, regs: KvmRegs) -> Result<(), Error> {
        match unsafe { kvm_set_regs(self.fd.as_raw_fd(), &regs) } {
            0 => Ok(()),
            _ => Err(Error::last_os_error()),
        }
    }
}

extern "C" {
    fn kvm_check_version(kvm: c_int, compat: *mut bool) -> c_int;
    fn kvm_max_vcpus(kvm: c_int, max: *mut usize) -> c_int;
    fn kvm_create_vm(kvm: c_int, fd: *mut c_int) -> c_int;
    fn kvm_get_vcpu_mmap_size(kvm: c_int) -> c_int;

    fn kvm_set_user_memory_region(
        vm: c_int,
        slot: u32,
        addr: u64,
        len: u64,
        mem: *mut c_void,
    ) -> c_int;
    fn kvm_create_vcpu(vm: c_int, id: c_int, fd: *mut c_int) -> c_int;

    fn kvm_run(vcpu: c_int) -> c_int;
    fn kvm_get_regs(vcpu: c_int, regs: *mut KvmRegs) -> c_int;
    fn kvm_set_regs(vcpu: c_int, regs: *const KvmRegs) -> c_int;
}

#[derive(Debug, Error)]
pub enum CreateVCpusError {
    #[error("failed to create vcpu #{1}")]
    CreateVcpuFailed(#[source] CreateVCpuError, u8),
}

#[derive(Debug, Error)]
pub enum CreateVCpuError {
    #[error("failed to create vcpu")]
    CreateVcpuFailed(#[source] Error),

    #[error("failed to mmap KVM_RUN")]
    MmapFailed(#[source] Error),
}
