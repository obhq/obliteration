use super::ffi::{kvm_get_regs, kvm_set_regs};
use super::regs::KvmRegs;
use super::run::KvmRun;
use crate::hv::{Cpu, CpuStates};
use libc::munmap;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, OwnedFd};
use thiserror::Error;

/// Implementation of [`Cpu`] for KVM.
pub struct KvmCpu<'a> {
    id: u32,
    fd: OwnedFd,
    cx: (*mut KvmRun, usize),
    vm: PhantomData<&'a OwnedFd>,
}

impl<'a> KvmCpu<'a> {
    /// # Safety
    /// - `cx` cannot be null and must be obtained from `mmap` on `fd`.
    /// - `len` must be the same value that used on `mmap`.
    pub unsafe fn new(id: u32, fd: OwnedFd, cx: *mut KvmRun, len: usize) -> Self {
        Self {
            id,
            fd,
            cx: (cx, len),
            vm: PhantomData,
        }
    }
}

impl<'a> Drop for KvmCpu<'a> {
    fn drop(&mut self) {
        use std::io::Error;

        if unsafe { munmap(self.cx.0.cast(), self.cx.1) } < 0 {
            panic!("failed to munmap kvm_run: {}", Error::last_os_error());
        };
    }
}

impl<'a> Cpu for KvmCpu<'a> {
    type GetStatesErr = GetStatesError;
    type SetStatesErr = SetStatesError;

    fn id(&self) -> usize {
        self.id.try_into().unwrap()
    }

    fn get_states(&mut self, states: &mut CpuStates) -> Result<(), Self::GetStatesErr> {
        use std::io::Error;

        // Get general purpose registers.
        let mut regs = MaybeUninit::uninit();
        let regs = match unsafe { kvm_get_regs(self.fd.as_raw_fd(), regs.as_mut_ptr()) } {
            0 => unsafe { regs.assume_init() },
            _ => return Err(GetStatesError::GetRegsFailed(Error::last_os_error())),
        };

        todo!()
    }

    fn set_states(&mut self, states: &CpuStates) -> Result<(), Self::SetStatesErr> {
        use std::io::Error;

        // Set general purpose registers.
        let mut regs = KvmRegs::default();

        match unsafe { kvm_set_regs(self.fd.as_raw_fd(), &regs) } {
            0 => {}
            _ => return Err(SetStatesError::SetRegsFailed(Error::last_os_error())),
        }

        todo!()
    }
}

/// Implementation of [`Cpu::GetStatesErr`].
#[derive(Debug, Error)]
pub enum GetStatesError {
    #[error("couldn't get general purpose registers")]
    GetRegsFailed(#[source] std::io::Error),
}

/// Implementation of [`Cpu::SetStatesErr`].
#[derive(Debug, Error)]
pub enum SetStatesError {
    #[error("couldn't set general purpose registers")]
    SetRegsFailed(#[source] std::io::Error),
}
