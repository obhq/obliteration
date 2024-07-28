use crate::vmm::{Cpu, CpuExit, CpuStates};
use hv_sys::hv_vcpu_destroy;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::num::NonZero;
use thiserror::Error;

macro_rules! wrap_return {
    ($ret:expr, $err:ident) => {
        match NonZero::new($ret) {
            Some(errno) => $err(errno),
            None => Ok(()),
        }
    };
}

/// Implementation of [`Cpu`] for Hypervisor Framework.
pub struct HfCpu<'a> {
    id: usize,
    instance: hv_vcpu_t,
    vm: PhantomData<&'a ()>,
}

impl<'a> HfCpu<'a> {
    pub fn new(id: usize, instance: hv_vcpu_t) -> Self {
        Self {
            id,
            instance,
            vm: PhantomData,
        }
    }
}

impl<'a> Drop for HfCpu<'a> {
    fn drop(&mut self) {
        let ret = unsafe { hv_vcpu_destroy(self.instance) };

        if ret != 0 {
            panic!("hv_vcpu_destroy() fails with {ret:#x}");
        }
    }
}

impl<'a> Cpu for HfCpu<'a> {
    type States<'b> = HfStates<'b> where Self: 'b;
    type GetStatesErr = GetStatesError;
    type Exit<'b> = HfExit<'b> where Self: 'b;
    type RunErr = RunError;

    fn id(&self) -> usize {
        self.id
    }

    fn states(&mut self) -> Result<Self::States<'_>, Self::GetStatesErr> {
        let mut rsp = MaybeUninit::new();
        let mut rip = MaybeUninit::new();
        let mut cr0 = MaybeUninit::new();
        let mut cr3 = MaybeUninit::new();
        let mut cr4 = MaybeUninit::new();
        let mut efer = MaybeUninit::new();
        let mut cs = MaybeUninit::new();
        let mut ds = MaybeUninit::new();
        let mut es = MaybeUninit::new();
        let mut fs = MaybeUninit::new();
        let mut gs = MaybeUninit::new();
        let mut ss = MaybeUninit::new();

        wrap_return!(
            hv_vcpu_read_register(self.instance, hv_sys::HV_X86_RSP, rsp.as_mut_ptr().cast()),
            GetStatesError::ReadRsp
        )?;
        wrap_return!(
            hv_vcpu_read_register(self.instance, hv_sys::HV_X86_RIP, rip.as_mut_ptr().cast()),
            GetStatesError::ReadRip
        )?;
        wrap_return!(
            hv_vcpu_read_register(self.instance, hv_sys::HV_X86_CR0, cr0.as_mut_ptr().cast()),
            GetStatesError::ReadCr0
        )?;
        wrap_return!(
            hv_vcpu_read_register(self.instance, hv_sys::HV_X86_CR3, cr3.as_mut_ptr().cast()),
            GetStatesError::ReadCr3
        )?;
        wrap_return!(
            hv_vcpu_read_register(self.instance, hv_sys::HV_X86_CR4, cr4.as_mut_ptr().cast()),
            GetStatesError::ReadCr4
        )?;
        wrap_return!(
            hv_vcpu_read_register(self.instance, hv_sys::HV_X86_EFER, efer.as_mut_ptr().cast()),
            GetStatesError::ReadEfer
        )?;
        wrap_return!(
            hv_vcpu_read_register(self.instance, hv_sys::HV_X86_CS, cs.as_mut_ptr().cast()),
            GetStatesError::ReadCs
        )?;
        wrap_return!(
            hv_vcpu_read_register(self.instance, hv_sys::HV_X86_DS, ds.as_mut_ptr().cast()),
            GetStatesError::ReadDs
        )?;
        wrap_return!(
            hv_vcpu_read_register(self.instance, hv_sys::HV_X86_ES, es.as_mut_ptr().cast()),
            GetStatesError::ReadEs
        )?;
        wrap_return!(
            hv_vcpu_read_register(self.instance, hv_sys::HV_X86_FS, fs.as_mut_ptr().cast()),
            GetStatesError::ReadFs
        )?;
        wrap_return!(
            hv_vcpu_read_register(self.instance, hv_sys::HV_X86_GS, gs.as_mut_ptr().cast()),
            GetStatesError::ReadGs
        )?;
        wrap_return!(
            hv_vcpu_read_register(self.instance, hv_sys::HV_X86_SS, ss.as_mut_ptr().cast()),
            GetStatesError::ReadSs
        )?;

        Ok(HfStates {
            cpu: PhantomData,
            dirty: false,
            rsp: unsafe { rsp.assume_init() },
            rip: unsafe { rip.assume_init() },
            cr0: unsafe { cr0.assume_init() },
            cr3: unsafe { cr3.assume_init() },
            cr4: unsafe { cr4.assume_init() },
            efer: unsafe { efer.assume_init() },
            cs: unsafe { cs.assume_init() },
            ds: unsafe { ds.assume_init() },
            es: unsafe { es.assume_init() },
            fs: unsafe { fs.assume_init() },
            gs: unsafe { gs.assume_init() },
            ss: unsafe { ss.assume_init() },
        })
    }

    fn run(&mut self) -> Result<Self::Exit<'_>, Self::RunErr> {
        wrap_return!(hv_sys::hv_vcpu_run(self.instance), RunError::Run)?;

        let exit_reason = MaybeUninit::new();

        wrap_return!(
            hv_sys::hv_vcpu_exit_info(self.instance, exit_reason.as_mut_ptr()),
            RunError::ReadExitReason
        )?;

        Ok(HfExit {
            cpu: PhantomData,
            exit_info: unsafe { exit_reason.assume_init() },
        })
    }
}

#[cfg(target_arch = "aarch64")]
type hv_vcpu_t = hv_sys::hv_vcpu_t;

#[cfg(target_arch = "x86_64")]
type hv_vcpu_t = hv_sys::hv_vcpuid_t;

/// Implementation of [`Cpu::States`] for Hypervisor Framework.
pub struct HfStates<'a> {
    cpu: PhantomData<&'a mut HfCpu<'a>>,
    dirty: bool,

    rsp: usize,
    rip: usize,

    cr0: usize,
    cr3: usize,
    cr4: usize,
    efer: usize,

    cs: usize,
    ds: usize,
    es: usize,
    fs: usize,
    gs: usize,
    ss: usize,
}

impl<'a> CpuStates for HfStates<'a> {
    #[cfg(target_arch = "x86_64")]
    fn set_rsp(&mut self, v: usize) {
        self.rsp = v;
        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_rip(&mut self, v: usize) {
        self.rip = v;
        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cr0(&mut self, v: usize) {
        self.cr0 = v;
        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cr3(&mut self, v: usize) {
        self.cr3 = v;
        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cr4(&mut self, v: usize) {
        self.cr4 = v;
        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_efer(&mut self, v: usize) {
        self.efer = v;
        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cs(&mut self, ty: u8, dpl: u8, p: bool, l: bool, d: bool) {
        let mut value = self.cs & 0xFFFF; // Preserve the selector
        value |= (ty as u64) << 40;
        value |= (dpl as u64) << 45;
        value |= (p as u64) << 47;
        value |= (l as u64) << 53;
        value |= (d as u64) << 54;

        self.cs = value;

        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_ds(&mut self, p: bool) {
        self.ds = p as usize;
        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_es(&mut self, p: bool) {
        self.es = p as usize;
        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_fs(&mut self, p: bool) {
        self.fs = p as usize;
        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_gs(&mut self, p: bool) {
        self.gs = p as usize;
        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_ss(&mut self, p: bool) {
        self.ss = p as usize;
        self.dirty = true;
    }
}

impl Drop for HfStates<'_> {
    fn drop(&mut self) {
        if !self.dirty {
            return;
        }

        todo!()
    }
}

/// Implementation of [`Cpu::Exit`] for Hypervisor Framework.
pub struct HfExit<'a> {
    cpu: PhantomData<&'a mut HfCpu<'a>>,
    exit_info: hv_sys::hv_vm_exitinfo_t,
}

impl<'a> CpuExit for HfExit<'a> {
    #[cfg(target_arch = "x86_64")]
    fn is_hlt(&self) -> bool {
        todo!()
    }
}

/// Implementation of [`Cpu::RunErr`].
#[derive(Debug, Error)]
pub enum RunError {
    #[error("error running vcpu: {0:#x}")]
    Run(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading exit reason: {0:#x}")]
    ReadExitReason(NonZero<hv_sys::hv_return_t>),
}

/// Implementation of [`Cpu::GetStatesErr`].
#[derive(Debug, Error)]
pub enum GetStatesError {
    #[error("error while reading rsp: {0:#x}")]
    ReadRsp(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading rip: {0:#x}")]
    ReadRip(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading cr0: {0:#x}")]
    ReadCr0(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading cr3: {0:#x}")]
    ReadCr3(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading cr4: {0:#x}")]
    ReadCr4(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading efer: {0:#x}")]
    ReadEfer(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading cs: {0:#x}")]
    ReadCs(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading ds: {0:#x}")]
    ReadDs(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading es: {0:#x}")]
    ReadEs(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading fs: {0:#x}")]
    ReadFs(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading gs: {0:#x}")]
    ReadGs(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading ss: {0:#x}")]
    ReadSs(NonZero<hv_sys::hv_return_t>),
}
