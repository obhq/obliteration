// SPDX-License-Identifier: MIT OR Apache-2.0
use super::ffi::{
    KvmFpu, KvmRegs, KvmSregs, KVM_GET_FPU, KVM_GET_REGS, KVM_GET_SREGS, KVM_SET_REGS,
};
use crate::vmm::hv::{CpuCommit, CpuStates, Rflags};
use libc::ioctl;
use std::ffi::c_int;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, OwnedFd};
use thiserror::Error;

/// Implementation of [`CpuStates`] for KVM.
pub struct KvmStates<'a> {
    cpu: &'a mut OwnedFd,
    gregs: KvmRegs,
    gdirty: bool,
    sregs: KvmSregs,
    sdirty: bool,
    fregs: KvmFpu,
}

impl<'a> KvmStates<'a> {
    pub fn from_cpu(cpu: &'a mut OwnedFd) -> Result<Self, StatesError> {
        use std::io::Error;

        // Load general purpose registers.
        let mut gregs = MaybeUninit::uninit();
        let gregs = if unsafe { ioctl(cpu.as_raw_fd(), KVM_GET_REGS, gregs.as_mut_ptr()) < 0 } {
            return Err(StatesError::GetGRegsFailed(Error::last_os_error()));
        } else {
            unsafe { gregs.assume_init() }
        };

        // Get special registers.
        let mut sregs = MaybeUninit::uninit();
        let sregs = if unsafe { ioctl(cpu.as_raw_fd(), KVM_GET_SREGS, sregs.as_mut_ptr()) < 0 } {
            return Err(StatesError::GetSRegsFailed(Error::last_os_error()));
        } else {
            unsafe { sregs.assume_init() }
        };

        // Get FPU registers.
        let mut fregs = MaybeUninit::uninit();
        let fregs = if unsafe { ioctl(cpu.as_raw_fd(), KVM_GET_FPU, fregs.as_mut_ptr()) < 0 } {
            return Err(StatesError::GetFRegsFailed(Error::last_os_error()));
        } else {
            unsafe { fregs.assume_init() }
        };

        Ok(KvmStates {
            cpu,
            gregs,
            gdirty: false,
            sregs,
            sdirty: false,
            fregs,
        })
    }
}

impl<'a> CpuStates for KvmStates<'a> {
    type Err = StatesError;

    fn get_rax(&mut self) -> Result<usize, Self::Err> {
        Ok(self.gregs.rax.try_into().unwrap())
    }

    fn get_rbx(&mut self) -> Result<usize, Self::Err> {
        Ok(self.gregs.rbx.try_into().unwrap())
    }

    fn get_rcx(&mut self) -> Result<usize, Self::Err> {
        Ok(self.gregs.rcx.try_into().unwrap())
    }

    fn get_rdx(&mut self) -> Result<usize, Self::Err> {
        Ok(self.gregs.rdx.try_into().unwrap())
    }

    fn get_rbp(&mut self) -> Result<usize, Self::Err> {
        Ok(self.gregs.rbp.try_into().unwrap())
    }

    fn get_r8(&mut self) -> Result<usize, Self::Err> {
        Ok(self.gregs.r8.try_into().unwrap())
    }

    fn get_r9(&mut self) -> Result<usize, Self::Err> {
        Ok(self.gregs.r9.try_into().unwrap())
    }

    fn get_r10(&mut self) -> Result<usize, Self::Err> {
        Ok(self.gregs.r10.try_into().unwrap())
    }

    fn get_r11(&mut self) -> Result<usize, Self::Err> {
        Ok(self.gregs.r11.try_into().unwrap())
    }

    fn get_r12(&mut self) -> Result<usize, Self::Err> {
        Ok(self.gregs.r12.try_into().unwrap())
    }

    fn get_r13(&mut self) -> Result<usize, Self::Err> {
        Ok(self.gregs.r13.try_into().unwrap())
    }

    fn get_r14(&mut self) -> Result<usize, Self::Err> {
        Ok(self.gregs.r14.try_into().unwrap())
    }

    fn get_r15(&mut self) -> Result<usize, Self::Err> {
        Ok(self.gregs.r15.try_into().unwrap())
    }

    fn get_rdi(&mut self) -> Result<usize, Self::Err> {
        Ok(self.gregs.rdi.try_into().unwrap())
    }

    fn set_rdi(&mut self, v: usize) {
        self.gregs.rdi = v.try_into().unwrap();
        self.gdirty = true;
    }

    fn get_rsi(&mut self) -> Result<usize, Self::Err> {
        Ok(self.gregs.rsi.try_into().unwrap())
    }

    fn set_rsi(&mut self, v: usize) {
        self.gregs.rsi = v.try_into().unwrap();
        self.gdirty = true;
    }

    fn get_rsp(&mut self) -> Result<usize, Self::Err> {
        Ok(self.gregs.rsp.try_into().unwrap())
    }

    fn set_rsp(&mut self, v: usize) {
        self.gregs.rsp = v.try_into().unwrap();
        self.gdirty = true;
    }

    fn get_rip(&mut self) -> Result<usize, Self::Err> {
        Ok(self.gregs.rip.try_into().unwrap())
    }

    fn set_rip(&mut self, v: usize) {
        self.gregs.rip = v.try_into().unwrap();
        self.gdirty = true;
    }

    fn set_cr0(&mut self, v: usize) {
        self.sregs.cr0 = v.try_into().unwrap();
        self.sdirty = true;
    }

    fn set_cr3(&mut self, v: usize) {
        self.sregs.cr3 = v.try_into().unwrap();
        self.sdirty = true;
    }

    fn set_cr4(&mut self, v: usize) {
        self.sregs.cr4 = v.try_into().unwrap();
        self.sdirty = true;
    }

    fn get_rflags(&mut self) -> Result<Rflags, Self::Err> {
        Ok(self.gregs.rflags.into())
    }

    fn set_efer(&mut self, v: usize) {
        self.sregs.efer = v.try_into().unwrap();
        self.sdirty = true;
    }

    fn get_cs(&mut self) -> Result<u16, Self::Err> {
        Ok(self.sregs.cs.selector)
    }

    fn set_cs(&mut self, ty: u8, dpl: u8, p: bool, l: bool, d: bool) {
        self.sregs.cs.ty = ty;
        self.sregs.cs.dpl = dpl;
        self.sregs.cs.present = p.into();
        self.sregs.cs.l = l.into();
        self.sregs.cs.db = d.into();
        self.sdirty = true;
    }

    fn get_ds(&mut self) -> Result<u16, Self::Err> {
        Ok(self.sregs.ds.selector)
    }

    fn set_ds(&mut self, p: bool) {
        self.sregs.ds.present = p.into();
        self.sdirty = true;
    }

    fn get_es(&mut self) -> Result<u16, Self::Err> {
        Ok(self.sregs.es.selector)
    }

    fn set_es(&mut self, p: bool) {
        self.sregs.es.present = p.into();
        self.sdirty = true;
    }

    fn get_fs(&mut self) -> Result<u16, Self::Err> {
        Ok(self.sregs.fs.selector)
    }

    fn set_fs(&mut self, p: bool) {
        self.sregs.fs.present = p.into();
        self.sdirty = true;
    }

    fn get_gs(&mut self) -> Result<u16, Self::Err> {
        Ok(self.sregs.gs.selector)
    }

    fn set_gs(&mut self, p: bool) {
        self.sregs.gs.present = p.into();
        self.sdirty = true;
    }

    fn get_ss(&mut self) -> Result<u16, Self::Err> {
        Ok(self.sregs.ss.selector)
    }

    fn set_ss(&mut self, p: bool) {
        self.sregs.ss.present = p.into();
        self.sdirty = true;
    }

    fn get_st0(&mut self) -> Result<[u8; 10], Self::Err> {
        Ok(self.fregs.fpr[0][..10].try_into().unwrap())
    }

    fn get_st1(&mut self) -> Result<[u8; 10], Self::Err> {
        Ok(self.fregs.fpr[1][..10].try_into().unwrap())
    }

    fn get_st2(&mut self) -> Result<[u8; 10], Self::Err> {
        Ok(self.fregs.fpr[2][..10].try_into().unwrap())
    }

    fn get_st3(&mut self) -> Result<[u8; 10], Self::Err> {
        Ok(self.fregs.fpr[3][..10].try_into().unwrap())
    }

    fn get_st4(&mut self) -> Result<[u8; 10], Self::Err> {
        Ok(self.fregs.fpr[4][..10].try_into().unwrap())
    }

    fn get_st5(&mut self) -> Result<[u8; 10], Self::Err> {
        Ok(self.fregs.fpr[5][..10].try_into().unwrap())
    }

    fn get_st6(&mut self) -> Result<[u8; 10], Self::Err> {
        Ok(self.fregs.fpr[6][..10].try_into().unwrap())
    }

    fn get_st7(&mut self) -> Result<[u8; 10], Self::Err> {
        Ok(self.fregs.fpr[7][..10].try_into().unwrap())
    }
}

impl<'a> CpuCommit for KvmStates<'a> {
    fn commit(self) -> Result<(), Self::Err> {
        use std::io::Error;

        // Set general purpose registers.
        if unsafe { self.gdirty && ioctl(self.cpu.as_raw_fd(), KVM_SET_REGS, &self.gregs) < 0 } {
            return Err(StatesError::SetGRegsFailed(Error::last_os_error()));
        }

        // Set special registers.
        if unsafe { self.sdirty && kvm_set_sregs(self.cpu.as_raw_fd(), &self.sregs) != 0 } {
            return Err(StatesError::SetSRegsFailed(Error::last_os_error()));
        }

        Ok(())
    }
}

/// Implementation of [`CpuStates::Err`].
#[derive(Debug, Error)]
pub enum StatesError {
    #[error("couldn't get general purpose registers")]
    GetGRegsFailed(#[source] std::io::Error),

    #[error("couldn't get special registers")]
    GetSRegsFailed(#[source] std::io::Error),

    #[error("couldn't get floating point registers")]
    GetFRegsFailed(#[source] std::io::Error),

    #[error("couldn't set general purpose registers")]
    SetGRegsFailed(#[source] std::io::Error),

    #[error("couldn't set special registers")]
    SetSRegsFailed(#[source] std::io::Error),
}

extern "C" {
    fn kvm_set_sregs(vcpu: c_int, regs: *const KvmSregs) -> c_int;
}
