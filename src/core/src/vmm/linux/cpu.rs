use super::ffi::{kvm_get_regs, kvm_get_sregs, kvm_run, kvm_set_regs, kvm_set_sregs};
use super::regs::{KvmRegs, KvmSpecialRegs};
use super::run::KvmRun;
use crate::vmm::{Cpu, CpuExit, CpuStates};
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
        assert!(len >= size_of::<KvmRun>());

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
    type States<'b> = KvmStates<'b> where Self: 'b;
    type GetStatesErr = GetStatesError;
    type Exit<'b> = KvmExit<'b> where Self: 'b;
    type RunErr = std::io::Error;

    fn id(&self) -> usize {
        self.id.try_into().unwrap()
    }

    fn states(&mut self) -> Result<Self::States<'_>, Self::GetStatesErr> {
        use std::io::Error;

        // Get general purpose registers.
        let mut gregs = MaybeUninit::uninit();
        let gregs = match unsafe { kvm_get_regs(self.fd.as_raw_fd(), gregs.as_mut_ptr()) } {
            0 => unsafe { gregs.assume_init() },
            _ => return Err(GetStatesError::GetGRegsFailed(Error::last_os_error())),
        };

        // Get special registers.
        let mut sregs = MaybeUninit::uninit();
        let sregs = match unsafe { kvm_get_sregs(self.fd.as_raw_fd(), sregs.as_mut_ptr()) } {
            0 => unsafe { sregs.assume_init() },
            _ => return Err(GetStatesError::GetSRegsFailed(Error::last_os_error())),
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
            0 => Ok(KvmExit {
                cx: unsafe { &*self.cx.0 },
            }),
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
    fn set_sp(&mut self, v: usize) {
        todo!()
    }

    #[cfg(target_arch = "aarch64")]
    fn set_pc(&mut self, v: usize) {
        todo!()
    }
}

impl<'a> Drop for KvmStates<'a> {
    fn drop(&mut self) {
        use std::io::Error;

        // Set general purpose registers.
        if unsafe { self.gdirty && kvm_set_regs(self.cpu.as_raw_fd(), &self.gregs) != 0 } {
            panic!(
                "couldn't set general purpose registers: {}",
                Error::last_os_error()
            );
        }

        // Set special registers.
        if unsafe { self.sdirty && kvm_set_sregs(self.cpu.as_raw_fd(), &self.sregs) != 0 } {
            panic!("couldn't set special registers: {}", Error::last_os_error());
        }
    }
}

/// Implementation of [`Cpu::Exit`] for KVM.
pub struct KvmExit<'a> {
    cx: &'a KvmRun,
}

impl<'a> CpuExit for KvmExit<'a> {
    #[cfg(target_arch = "x86_64")]
    fn reason(&mut self) -> crate::vmm::ExitReason {
        match self.cx.exit_reason {
            2 => {
                // Check direction.
                let io = unsafe { &self.cx.exit.io };
                let port = io.port;
                let data = unsafe { (self.cx as *const KvmRun as *const u8).add(io.data_offset) };
                let len: usize = io.size.into();

                match io.direction {
                    0 => todo!(), // KVM_EXIT_IO_IN
                    1 => crate::vmm::ExitReason::IoOut(port, unsafe {
                        std::slice::from_raw_parts(data, len)
                    }),
                    _ => unreachable!(),
                }
            }
            5 => crate::vmm::ExitReason::Hlt,
            reason => todo!("unhandled exit reason: {}", reason),
        }
    }
}

/// Implementation of [`Cpu::GetStatesErr`].
#[derive(Debug, Error)]
pub enum GetStatesError {
    #[error("couldn't get general purpose registers")]
    GetGRegsFailed(#[source] std::io::Error),

    #[error("couldn't get special registers")]
    GetSRegsFailed(#[source] std::io::Error),
}
