use super::ffi::{
    kvm_get_regs, kvm_get_sregs, kvm_run, kvm_set_regs, kvm_set_sregs, kvm_translate,
    KvmTranslation,
};
use super::regs::{KvmRegs, KvmSpecialRegs};
use super::run::KvmRun;
use crate::vmm::hv::{Cpu, CpuExit, CpuIo, CpuStates, IoBuf};
use libc::munmap;
use std::error::Error;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, OwnedFd};
use thiserror::Error;

/// Implementation of [`Cpu`] for KVM.
pub struct KvmCpu<'a> {
    fd: OwnedFd,
    cx: (*mut KvmRun, usize),
    vm: PhantomData<&'a OwnedFd>,
}

impl<'a> KvmCpu<'a> {
    /// # Safety
    /// - `cx` cannot be null and must be obtained from `mmap` on `fd`.
    /// - `len` must be the same value that used on `mmap`.
    pub unsafe fn new(fd: OwnedFd, cx: *mut KvmRun, len: usize) -> Self {
        assert!(len >= size_of::<KvmRun>());

        Self {
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
    type States<'b> = KvmStates<'b> where Self: 'b;
    type GetStatesErr = StatesError;
    type Exit<'b> = KvmExit<'b, 'a> where Self: 'b;
    type RunErr = std::io::Error;

    fn states(&mut self) -> Result<Self::States<'_>, Self::GetStatesErr> {
        use std::io::Error;

        // Get general purpose registers.
        let mut gregs = MaybeUninit::uninit();
        let gregs = match unsafe { kvm_get_regs(self.fd.as_raw_fd(), gregs.as_mut_ptr()) } {
            0 => unsafe { gregs.assume_init() },
            _ => return Err(StatesError::GetGRegsFailed(Error::last_os_error())),
        };

        // Get special registers.
        let mut sregs = MaybeUninit::uninit();
        let sregs = match unsafe { kvm_get_sregs(self.fd.as_raw_fd(), sregs.as_mut_ptr()) } {
            0 => unsafe { sregs.assume_init() },
            _ => return Err(StatesError::GetSRegsFailed(Error::last_os_error())),
        };

        Ok(KvmStates {
            cpu: &mut self.fd,
            gregs,
            gdirty: false,
            sregs,
            sdirty: false,
        })
    }

    fn run(&mut self) -> Result<Self::Exit<'_>, Self::RunErr> {
        match unsafe { kvm_run(self.fd.as_raw_fd()) } {
            0 => Ok(KvmExit(self)),
            _ => Err(std::io::Error::last_os_error()),
        }
    }
}

/// Implementation of [`Cpu::States`] for KVM.
pub struct KvmStates<'a> {
    cpu: &'a mut OwnedFd,
    gregs: KvmRegs,
    gdirty: bool,
    sregs: KvmSpecialRegs,
    sdirty: bool,
}

impl<'a> CpuStates for KvmStates<'a> {
    type Err = StatesError;

    #[cfg(target_arch = "x86_64")]
    fn set_rdi(&mut self, v: usize) {
        self.gregs.rdi = v;
        self.gdirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_rsp(&mut self, v: usize) {
        self.gregs.rsp = v;
        self.gdirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_rip(&mut self, v: usize) {
        self.gregs.rip = v;
        self.gdirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cr0(&mut self, v: usize) {
        self.sregs.cr0 = v;
        self.sdirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cr3(&mut self, v: usize) {
        self.sregs.cr3 = v;
        self.sdirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cr4(&mut self, v: usize) {
        self.sregs.cr4 = v;
        self.sdirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_efer(&mut self, v: usize) {
        self.sregs.efer = v;
        self.sdirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cs(&mut self, ty: u8, dpl: u8, p: bool, l: bool, d: bool) {
        self.sregs.cs.ty = ty;
        self.sregs.cs.dpl = dpl;
        self.sregs.cs.present = p.into();
        self.sregs.cs.l = l.into();
        self.sregs.cs.db = d.into();
        self.sdirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_ds(&mut self, p: bool) {
        self.sregs.ds.present = p.into();
        self.sdirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_es(&mut self, p: bool) {
        self.sregs.es.present = p.into();
        self.sdirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_fs(&mut self, p: bool) {
        self.sregs.fs.present = p.into();
        self.sdirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_gs(&mut self, p: bool) {
        self.sregs.gs.present = p.into();
        self.sdirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_ss(&mut self, p: bool) {
        self.sregs.ss.present = p.into();
        self.sdirty = true;
    }

    #[cfg(target_arch = "aarch64")]
    fn set_sp_el1(&mut self, v: usize) {
        todo!()
    }

    #[cfg(target_arch = "aarch64")]
    fn set_pc(&mut self, v: usize) {
        todo!()
    }

    fn commit(self) -> Result<(), Self::Err> {
        use std::io::Error;

        // Set general purpose registers.
        if unsafe { self.gdirty && kvm_set_regs(self.cpu.as_raw_fd(), &self.gregs) != 0 } {
            return Err(StatesError::SetGRegsFailed(Error::last_os_error()));
        }

        // Set special registers.
        if unsafe { self.sdirty && kvm_set_sregs(self.cpu.as_raw_fd(), &self.sregs) != 0 } {
            return Err(StatesError::SetSRegsFailed(Error::last_os_error()));
        }

        Ok(())
    }
}

/// Implementation of [`Cpu::Exit`] for KVM.
pub struct KvmExit<'a, 'b>(&'a mut KvmCpu<'b>);

impl<'a, 'b> CpuExit for KvmExit<'a, 'b> {
    type Io = KvmIo<'a, 'b>;

    #[cfg(target_arch = "x86_64")]
    fn into_hlt(self) -> Result<(), Self> {
        if unsafe { (*self.0.cx.0).exit_reason == 5 } {
            Ok(())
        } else {
            Err(self)
        }
    }

    fn into_io(self) -> Result<Self::Io, Self> {
        if unsafe { (*self.0.cx.0).exit_reason } == 6 {
            Ok(KvmIo(self.0))
        } else {
            Err(self)
        }
    }
}

/// Implementation of [`CpuIo`] for KVM.
pub struct KvmIo<'a, 'b>(&'a mut KvmCpu<'b>);

impl<'a, 'b> CpuIo for KvmIo<'a, 'b> {
    fn addr(&self) -> usize {
        unsafe { (*self.0.cx.0).exit.mmio.phys_addr }
    }

    fn buffer(&mut self) -> IoBuf {
        let io = unsafe { &mut (*self.0.cx.0).exit.mmio };
        let len: usize = io.len.try_into().unwrap();
        let buf = &mut io.data[..len];

        match io.is_write {
            0 => IoBuf::Read(buf),
            _ => IoBuf::Write(buf),
        }
    }

    fn translate(&self, vaddr: usize) -> Result<usize, Box<dyn Error>> {
        let mut data = KvmTranslation {
            linear_address: vaddr,
            physical_address: 0,
            valid: 0,
            writeable: 0,
            usermode: 0,
            pad: [0; 5],
        };

        match unsafe { kvm_translate(self.0.fd.as_raw_fd(), &mut data) } {
            0 => Ok(data.physical_address),
            _ => return Err(Box::new(std::io::Error::last_os_error())),
        }
    }
}

/// Implementation of [`Cpu::GetStatesErr`] and [`CpuStates::Err`].
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
