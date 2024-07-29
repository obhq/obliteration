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

macro_rules! wrap_return {
    ($ret:expr) => {
        match NonZero::new($ret) {
            Some(errno) => Err(errno),
            None => Ok(()),
        }
    };

    ($ret:expr, $err:path) => {
        match NonZero::new($ret) {
            Some(errno) => Err($err(errno)),
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

    #[cfg(target_arch = "x86_64")]
    fn read_register(
        &self,
        register: hv_sys::hv_x86_reg_t,
    ) -> Result<usize, NonZero<hv_sys::hv_return_t>> {
        let mut value = MaybeUninit::<usize>::uninit();

        wrap_return!(unsafe {
            hv_sys::hv_vcpu_read_register(self.instance, register, value.as_mut_ptr().cast())
        })?;

        Ok(unsafe { value.assume_init() })
    }

    #[cfg(target_arch = "aarch64")]
    fn read_register(
        &self,
        register: hv_sys::hv_reg_t,
    ) -> Result<usize, NonZero<hv_sys::hv_return_t>> {
        let mut value = MaybeUninit::<usize>::uninit();

        wrap_return!(unsafe {
            hv_sys::hv_vcpu_get_reg(self.instance, register, value.as_mut_ptr().cast())
        })?;

        Ok(unsafe { value.assume_init() })
    }

    #[cfg(target_arch = "x86_64")]
    fn write_register(
        &self,
        register: hv_sys::hv_x86_reg_t,
        value: usize,
    ) -> Result<(), NonZero<hv_sys::hv_return_t>> {
        wrap_return!(unsafe {
            hv_sys::hv_vcpu_write_register(self.instance, register, value as u64)
        })
    }
}

impl<'a> Cpu for HfCpu<'a> {
    type States<'b> = HfStates<'b, 'a> where Self: 'b;
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
            cpu: self,
            dirty: false,

            rsp,
            rip,
            cr0,
            cr3,
            cr4,

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
        wrap_return!(
            unsafe { hv_sys::hv_vcpu_run_until(self.instance, hv_sys::HV_DEADLINE_FOREVER) },
            RunError::Run
        )?;

        let mut exit_reason = MaybeUninit::uninit();

        wrap_return!(
            unsafe { hv_sys::hv_vcpu_exit_info(self.instance, exit_reason.as_mut_ptr()) },
            RunError::ReadExitReason
        )?;

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

impl<'a> Drop for HfCpu<'a> {
    fn drop(&mut self) {
        let ret = unsafe { hv_vcpu_destroy(self.instance) };

        if ret != 0 {
            panic!("hv_vcpu_destroy() fails with {ret:#x}");
        }
    }
}

/// Implementation of [`Cpu::States`] for Hypervisor Framework.
pub struct HfStates<'a, 'b> {
    cpu: &'a mut HfCpu<'b>,
    dirty: bool,

    rsp: usize,
    rip: usize,

    cr0: usize,
    cr3: usize,
    cr4: usize,

    ds: usize,
    es: usize,
    fs: usize,
    gs: usize,
    ss: usize,
}

impl<'a, 'b> CpuStates for HfStates<'a, 'b> {
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
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn set_es(&mut self, p: bool) {
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn set_fs(&mut self, p: bool) {
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn set_gs(&mut self, p: bool) {
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn set_ss(&mut self, p: bool) {
        todo!()
    }
}

impl<'a, 'b> Drop for HfStates<'a, 'b> {
    fn drop(&mut self) {
        if !self.dirty {
            return;
        }

        self.cpu
            .write_register(hv_sys::hv_x86_reg_t_HV_X86_RIP, self.rip)
            .unwrap();
        self.cpu
            .write_register(hv_sys::hv_x86_reg_t_HV_X86_RSP, self.rsp)
            .unwrap();
        self.cpu
            .write_register(hv_sys::hv_x86_reg_t_HV_X86_CR0, self.cr0)
            .unwrap();
        self.cpu
            .write_register(hv_sys::hv_x86_reg_t_HV_X86_CR3, self.cr3)
            .unwrap();
        self.cpu
            .write_register(hv_sys::hv_x86_reg_t_HV_X86_CR4, self.cr4)
            .unwrap();
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
    #[cfg(target_arch = "x86_64")]
    #[error("error while reading rsp: {0:#x}")]
    ReadRsp(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "x86_64")]
    #[error("error while reading rip: {0:#x}")]
    ReadRip(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "x86_64")]
    #[error("error while reading cr0: {0:#x}")]
    ReadCr0(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "x86_64")]
    #[error("error while reading cr3: {0:#x}")]
    ReadCr3(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "x86_64")]
    #[error("error while reading cr4: {0:#x}")]
    ReadCr4(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "x86_64")]
    #[error("error while reading ds: {0:#x}")]
    ReadDs(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "x86_64")]
    #[error("error while reading es: {0:#x}")]
    ReadEs(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "x86_64")]
    #[error("error while reading fs: {0:#x}")]
    ReadFs(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "x86_64")]
    #[error("error while reading gs: {0:#x}")]
    ReadGs(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "x86_64")]
    #[error("error while reading ss: {0:#x}")]
    ReadSs(NonZero<hv_sys::hv_return_t>),
}
