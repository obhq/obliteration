// SPDX-License-Identifier: MIT OR Apache-2.0
use super::ffi::{
    KVM_GET_FPU, KVM_GET_REGS, KVM_GET_SREGS, KVM_SET_REGS, KVM_SET_SREGS, KvmFpu, KvmRegs,
    KvmSregs,
};
use super::{HvError, Kvm};
use crate::{CpuCommit, CpuStates, FeatLeaf, HypervisorExt};
use libc::ioctl;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, OwnedFd};
use thiserror::Error;
use x86_64::{Efer, Rflags};

impl HypervisorExt for Kvm {
    fn set_cpuid(&mut self, leaf: FeatLeaf) -> Result<(), HvError> {
        match self.feats.iter_mut().find(|f| f.id == leaf.id) {
            Some(f) => *f = leaf,
            None => self.feats.push(leaf),
        }

        Ok(())
    }
}

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
            return Err(StatesError::GetGeneral(Error::last_os_error()));
        } else {
            unsafe { gregs.assume_init() }
        };

        // Get special registers.
        let mut sregs = MaybeUninit::uninit();
        let sregs = if unsafe { ioctl(cpu.as_raw_fd(), KVM_GET_SREGS, sregs.as_mut_ptr()) < 0 } {
            return Err(StatesError::GetSpecial(Error::last_os_error()));
        } else {
            unsafe { sregs.assume_init() }
        };

        // Get FPU registers.
        let mut fregs = MaybeUninit::uninit();
        let fregs = if unsafe { ioctl(cpu.as_raw_fd(), KVM_GET_FPU, fregs.as_mut_ptr()) < 0 } {
            return Err(StatesError::GetFp(Error::last_os_error()));
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

impl CpuStates for KvmStates<'_> {
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

    fn set_rdx(&mut self, v: usize) {
        self.gregs.rdx = v.try_into().unwrap();
        self.gdirty = true;
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

    fn set_rflags(&mut self, v: Rflags) {
        self.gregs.rflags = v.into_bits();
        self.gdirty = true;
    }

    fn set_efer(&mut self, v: Efer) {
        self.sregs.efer = v.into_bits();
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

    fn get_fcw(&mut self) -> Result<u32, Self::Err> {
        Ok(self.fregs.fcw.into())
    }

    fn get_fsw(&mut self) -> Result<u32, Self::Err> {
        Ok(self.fregs.fsw.into())
    }

    fn get_ftwx(&mut self) -> Result<u32, Self::Err> {
        Ok(self.fregs.ftwx.into())
    }

    fn get_fiseg(&mut self) -> Result<u32, Self::Err> {
        Ok((self.fregs.last_ip >> 32) as u32)
    }

    fn get_fioff(&mut self) -> Result<u32, Self::Err> {
        Ok((self.fregs.last_ip & 0xFFFFFFFF) as u32)
    }

    fn get_foseg(&mut self) -> Result<u32, Self::Err> {
        Ok((self.fregs.last_dp >> 32) as u32)
    }

    fn get_fooff(&mut self) -> Result<u32, Self::Err> {
        Ok((self.fregs.last_dp & 0xFFFFFFFF) as u32)
    }

    fn get_fop(&mut self) -> Result<u32, Self::Err> {
        Ok(self.fregs.last_opcode.into())
    }

    fn get_xmm0(&mut self) -> Result<u128, Self::Err> {
        Ok(u128::from_le_bytes(self.fregs.xmm[0]))
    }

    fn get_xmm1(&mut self) -> Result<u128, Self::Err> {
        Ok(u128::from_le_bytes(self.fregs.xmm[1]))
    }

    fn get_xmm2(&mut self) -> Result<u128, Self::Err> {
        Ok(u128::from_le_bytes(self.fregs.xmm[2]))
    }

    fn get_xmm3(&mut self) -> Result<u128, Self::Err> {
        Ok(u128::from_le_bytes(self.fregs.xmm[3]))
    }

    fn get_xmm4(&mut self) -> Result<u128, Self::Err> {
        Ok(u128::from_le_bytes(self.fregs.xmm[4]))
    }

    fn get_xmm5(&mut self) -> Result<u128, Self::Err> {
        Ok(u128::from_le_bytes(self.fregs.xmm[5]))
    }

    fn get_xmm6(&mut self) -> Result<u128, Self::Err> {
        Ok(u128::from_le_bytes(self.fregs.xmm[6]))
    }

    fn get_xmm7(&mut self) -> Result<u128, Self::Err> {
        Ok(u128::from_le_bytes(self.fregs.xmm[7]))
    }

    fn get_xmm8(&mut self) -> Result<u128, Self::Err> {
        Ok(u128::from_le_bytes(self.fregs.xmm[8]))
    }

    fn get_xmm9(&mut self) -> Result<u128, Self::Err> {
        Ok(u128::from_le_bytes(self.fregs.xmm[9]))
    }

    fn get_xmm10(&mut self) -> Result<u128, Self::Err> {
        Ok(u128::from_le_bytes(self.fregs.xmm[10]))
    }

    fn get_xmm11(&mut self) -> Result<u128, Self::Err> {
        Ok(u128::from_le_bytes(self.fregs.xmm[11]))
    }

    fn get_xmm12(&mut self) -> Result<u128, Self::Err> {
        Ok(u128::from_le_bytes(self.fregs.xmm[12]))
    }

    fn get_xmm13(&mut self) -> Result<u128, Self::Err> {
        Ok(u128::from_le_bytes(self.fregs.xmm[13]))
    }

    fn get_xmm14(&mut self) -> Result<u128, Self::Err> {
        Ok(u128::from_le_bytes(self.fregs.xmm[14]))
    }

    fn get_xmm15(&mut self) -> Result<u128, Self::Err> {
        Ok(u128::from_le_bytes(self.fregs.xmm[15]))
    }

    fn get_mxcsr(&mut self) -> Result<u32, Self::Err> {
        Ok(self.fregs.mxcsr)
    }
}

impl CpuCommit for KvmStates<'_> {
    fn commit(self) -> Result<(), Self::Err> {
        use std::io::Error;

        // Set general purpose registers.
        if unsafe { self.gdirty && ioctl(self.cpu.as_raw_fd(), KVM_SET_REGS, &self.gregs) < 0 } {
            return Err(StatesError::SetGeneral(Error::last_os_error()));
        }

        // Set special registers.
        if unsafe { self.sdirty && ioctl(self.cpu.as_raw_fd(), KVM_SET_SREGS, &self.sregs) < 0 } {
            return Err(StatesError::SetSpecial(Error::last_os_error()));
        }

        Ok(())
    }
}

/// Implementation of [`CpuStates::Err`].
#[derive(Debug, Error)]
pub enum StatesError {
    #[error("couldn't get general purpose registers")]
    GetGeneral(#[source] std::io::Error),

    #[error("couldn't get special registers")]
    GetSpecial(#[source] std::io::Error),

    #[error("couldn't get floating point registers")]
    GetFp(#[source] std::io::Error),

    #[error("couldn't set general purpose registers")]
    SetGeneral(#[source] std::io::Error),

    #[error("couldn't set special registers")]
    SetSpecial(#[source] std::io::Error),
}
