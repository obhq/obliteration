// SPDX-License-Identifier: MIT OR Apache-2.0
use super::ffi::{KvmRegs, KVM_GET_REGS, KVM_SET_REGS};
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
    sregs: SpecialRegs,
    sdirty: bool,
}

impl<'a> KvmStates<'a> {
    pub fn from_cpu(cpu: &'a mut OwnedFd) -> Result<Self, StatesError> {
        use std::io::Error;

        // Load general purpose registers.
        let mut gregs = MaybeUninit::uninit();
        let gregs = if unsafe { ioctl(cpu.as_raw_fd(), KVM_GET_REGS, gregs.as_mut_ptr()) } < 0 {
            return Err(StatesError::GetGRegsFailed(Error::last_os_error()));
        } else {
            unsafe { gregs.assume_init() }
        };

        // Get special registers.
        let mut sregs = MaybeUninit::uninit();
        let sregs = match unsafe { kvm_get_sregs(cpu.as_raw_fd(), sregs.as_mut_ptr()) } {
            0 => unsafe { sregs.assume_init() },
            _ => return Err(StatesError::GetSRegsFailed(Error::last_os_error())),
        };

        Ok(KvmStates {
            cpu,
            gregs,
            gdirty: false,
            sregs,
            sdirty: false,
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
        self.sregs.cr0 = v;
        self.sdirty = true;
    }

    fn set_cr3(&mut self, v: usize) {
        self.sregs.cr3 = v;
        self.sdirty = true;
    }

    fn set_cr4(&mut self, v: usize) {
        self.sregs.cr4 = v;
        self.sdirty = true;
    }

    fn get_rflags(&mut self) -> Result<Rflags, Self::Err> {
        Ok(self.gregs.rflags.into())
    }

    fn set_efer(&mut self, v: usize) {
        self.sregs.efer = v;
        self.sdirty = true;
    }

    fn set_cs(&mut self, ty: u8, dpl: u8, p: bool, l: bool, d: bool) {
        self.sregs.cs.ty = ty;
        self.sregs.cs.dpl = dpl;
        self.sregs.cs.present = p.into();
        self.sregs.cs.l = l.into();
        self.sregs.cs.db = d.into();
        self.sdirty = true;
    }

    fn set_ds(&mut self, p: bool) {
        self.sregs.ds.present = p.into();
        self.sdirty = true;
    }

    fn set_es(&mut self, p: bool) {
        self.sregs.es.present = p.into();
        self.sdirty = true;
    }

    fn set_fs(&mut self, p: bool) {
        self.sregs.fs.present = p.into();
        self.sdirty = true;
    }

    fn set_gs(&mut self, p: bool) {
        self.sregs.gs.present = p.into();
        self.sdirty = true;
    }

    fn set_ss(&mut self, p: bool) {
        self.sregs.ss.present = p.into();
        self.sdirty = true;
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

/// Implementation of `kvm_sregs` structure.
#[repr(C)]
struct SpecialRegs {
    pub cs: Segment,
    pub ds: Segment,
    pub es: Segment,
    pub fs: Segment,
    pub gs: Segment,
    pub ss: Segment,
    pub tr: Segment,
    pub ldt: Segment,
    pub gdt: DTable,
    pub idt: DTable,
    pub cr0: usize,
    pub cr2: u64,
    pub cr3: usize,
    pub cr4: usize,
    pub cr8: u64,
    pub efer: usize,
    pub apic_base: u64,
    pub interrupt_bitmap: [u64; 4],
}

/// Implementation of `kvm_segment` structure.
#[repr(C)]
pub struct Segment {
    pub base: u64,
    pub limit: u32,
    pub selector: u16,
    pub ty: u8,
    pub present: u8,
    pub dpl: u8,
    pub db: u8,
    pub s: u8,
    pub l: u8,
    pub g: u8,
    pub avl: u8,
    pub unusable: u8,
    pub padding: u8,
}

/// Implementation of `kvm_dtable` structure.
#[repr(C)]
struct DTable {
    base: u64,
    limit: u16,
    padding: [u16; 3],
}

/// Implementation of [`CpuStates::Err`].
#[derive(Debug, Error)]
pub enum StatesError {
    #[error("couldn't get general purpose registers")]
    GetGRegsFailed(#[source] std::io::Error),

    #[error("couldn't get special registers")]
    GetSRegsFailed(#[source] std::io::Error),

    #[error("couldn't set general purpose registers")]
    SetGRegsFailed(#[source] std::io::Error),

    #[error("couldn't set special registers")]
    SetSRegsFailed(#[source] std::io::Error),
}

extern "C" {
    fn kvm_get_sregs(vcpu: c_int, regs: *mut SpecialRegs) -> c_int;
    fn kvm_set_sregs(vcpu: c_int, regs: *const SpecialRegs) -> c_int;
}
