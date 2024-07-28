use crate::vmm::{Cpu, CpuExit, CpuStates};
use hv_sys::hv_vcpu_destroy;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::num::NonZero;
use thiserror::Error;

#[cfg(target_arch = "aarch64")]
#[allow(non_camel_case_types)]
type hv_vcpu_t = hv_sys::hv_vcpu_t;

#[cfg(target_arch = "x86_64")]
#[allow(non_camel_case_types)]
type hv_vcpu_t = hv_sys::hv_vcpuid_t;

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

    #[cfg(target_arch = "x86_64")]
    fn states(&mut self) -> Result<Self::States<'_>, Self::GetStatesErr> {
        let rsp = self
            .read_register(hv_sys::hv_x86_reg_t_HV_X86_RIP)
            .map_err(GetStatesError::ReadRsp)?;
        let rip = self
            .read_register(hv_sys::hv_x86_reg_t_HV_X86_RSP)
            .map_err(GetStatesError::ReadRip)?;
        let cr0 = self
            .read_register(hv_sys::hv_x86_reg_t_HV_X86_CR0)
            .map_err(GetStatesError::ReadCr0)?;
        let cr3 = self
            .read_register(hv_sys::hv_x86_reg_t_HV_X86_CR3)
            .map_err(GetStatesError::ReadCr3)?;
        let cr4 = self
            .read_register(hv_sys::hv_x86_reg_t_HV_X86_CR4)
            .map_err(GetStatesError::ReadCr4)?;
        /*let efer = self
        .read_register(hv_sys::hv_x86_reg_t_HV_X86_EFER)
        .map_err(GetStatesError::ReadEfer)?;*/
        let cs = self
            .read_register(hv_sys::hv_x86_reg_t_HV_X86_CS)
            .map_err(GetStatesError::ReadCs)?;
        let ds = self
            .read_register(hv_sys::hv_x86_reg_t_HV_X86_DS)
            .map_err(GetStatesError::ReadDs)?;
        let es = self
            .read_register(hv_sys::hv_x86_reg_t_HV_X86_ES)
            .map_err(GetStatesError::ReadEs)?;
        let fs = self
            .read_register(hv_sys::hv_x86_reg_t_HV_X86_FS)
            .map_err(GetStatesError::ReadFs)?;
        let gs = self
            .read_register(hv_sys::hv_x86_reg_t_HV_X86_GS)
            .map_err(GetStatesError::ReadGs)?;
        let ss = self
            .read_register(hv_sys::hv_x86_reg_t_HV_X86_SS)
            .map_err(GetStatesError::ReadSs)?;

        Ok(HfStates {
            cpu: PhantomData,
            dirty: false,

            rsp,
            rip,
            cr0,
            cr3,
            cr4,
            /*efer,*/
            cs,
            ds,
            es,
            fs,
            gs,
            ss,
        })
    }

    #[cfg(target_arch = "aarch64")]
    fn states(&mut self) -> Result<Self::States<'_>, Self::GetStatesErr> {
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn run(&mut self) -> Result<Self::Exit<'_>, Self::RunErr> {
        if let Some(err) = NonZero::new(unsafe { hv_sys::hv_vcpu_run(self.instance) }) {
            return Err(RunError::Run(err));
        };

        let mut exit_reason = MaybeUninit::uninit();

        if let Some(err) = NonZero::new(unsafe {
            hv_sys::hv_vcpu_exit_info(self.instance, exit_reason.as_mut_ptr())
        }) {
            return Err(RunError::ReadExitReason(err));
        };

        Ok(HfExit {
            cpu: PhantomData,
            exit_info: unsafe { exit_reason.assume_init() },
        })
    }

    #[cfg(target_arch = "aarch64")]
    fn run(&mut self) -> Result<Self::Exit<'_>, Self::RunErr> {
        todo!()
    }
}

impl HfCpu<'_> {
    #[cfg(target_arch = "x86_64")]
    fn read_register(
        &self,
        register: hv_sys::hv_x86_reg_t,
    ) -> Result<usize, NonZero<hv_sys::hv_return_t>> {
        let mut value = MaybeUninit::<usize>::uninit();

        if let Some(err) = NonZero::new(unsafe {
            hv_sys::hv_vcpu_read_register(self.instance, register, value.as_mut_ptr().cast())
        }) {
            return Err(err);
        }

        Ok(unsafe { value.assume_init() })
    }

    #[cfg(target_arch = "aarch64")]
    fn read_register(
        &self,
        register: hv_sys::hv_reg_t,
    ) -> Result<usize, NonZero<hv_sys::hv_return_t>> {
        let mut value = MaybeUninit::<usize>::uninit();

        if let Some(err) = NonZero::new(unsafe {
            hv_sys::hv_vcpu_get_reg(self.instance, register, value.as_mut_ptr().cast())
        }) {
            return Err(err);
        }

        Ok(unsafe { value.assume_init() })
    }
}

/// Implementation of [`Cpu::States`] for Hypervisor Framework.
pub struct HfStates<'a> {
    cpu: PhantomData<&'a mut HfCpu<'a>>,
    dirty: bool,

    rsp: usize,
    rip: usize,

    cr0: usize,
    cr3: usize,
    cr4: usize,
    //efer: usize,
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
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cs(&mut self, ty: u8, dpl: u8, p: bool, l: bool, d: bool) {
        todo!()
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
    #[cfg(target_arch = "x86_64")]
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
