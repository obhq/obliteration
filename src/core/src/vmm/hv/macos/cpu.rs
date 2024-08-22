use crate::vmm::hv::{Cpu, CpuExit, CpuIo, CpuStates, IoBuf};
use hv_sys::hv_vcpu_destroy;
use std::error::Error;
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
    instance: hv_vcpu_t,

    #[cfg(target_arch = "aarch64")]
    exit: *const hv_sys::hv_vcpu_exit_t,

    vm: PhantomData<&'a ()>,
}

impl<'a> HfCpu<'a> {
    #[cfg(target_arch = "x86_64")]
    pub fn new_x64(instance: hv_vcpu_t) -> Self {
        Self {
            instance,
            vm: PhantomData,
        }
    }

    #[cfg(target_arch = "aarch64")]
    pub fn new_aarch64(instance: hv_vcpu_t, exit: *const hv_sys::hv_vcpu_exit_t) -> Self {
        Self {
            instance,
            exit,
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
        &mut self,
        register: hv_sys::hv_x86_reg_t,
        value: usize,
    ) -> Result<(), NonZero<hv_sys::hv_return_t>> {
        wrap_return!(unsafe {
            hv_sys::hv_vcpu_write_register(self.instance, register, value as u64)
        })
    }

    #[cfg(target_arch = "x86_64")]
    fn write_vmcs(&mut self, field: u32, value: u64) -> Result<(), NonZero<hv_sys::hv_return_t>> {
        wrap_return!(unsafe { hv_sys::hv_vmx_vcpu_write_vmcs(self.instance, field, value) })
    }
}

impl<'a> Cpu for HfCpu<'a> {
    type States<'b> = HfStates<'b, 'a> where Self: 'b;
    type GetStatesErr = StatesError;
    type Exit<'b> = HfExit<'b> where Self: 'b;
    type RunErr = RunError;

    #[cfg(target_arch = "x86_64")]
    fn states(&mut self) -> Result<Self::States<'_>, Self::GetStatesErr> {
        Ok(HfStates {
            cpu: self,
            rsp: State::None,
            rip: State::None,
            cr0: State::None,
            cr3: State::None,
            cr4: State::None,
            cs: State::None,
            ds: State::None,
            es: State::None,
            fs: State::None,
            gs: State::None,
            ss: State::None,
        })
    }

    #[cfg(target_arch = "aarch64")]
    fn states(&mut self) -> Result<Self::States<'_>, Self::GetStatesErr> {
        Ok(HfStates {
            cpu: self,
            tcr_el1: State::None,
            ttbr1_el1: State::None,
            sp_el1: State::None,
        })
    }

    #[cfg(target_arch = "x86_64")]
    fn run(&mut self) -> Result<Self::Exit<'_>, Self::RunErr> {
        wrap_return!(
            unsafe { hv_sys::hv_vcpu_run_until(self.instance, hv_sys::HV_DEADLINE_FOREVER) },
            RunError::Run
        )?;

        let mut exit_reason = 0u64;

        wrap_return!(
            unsafe {
                hv_sys::hv_vmx_vcpu_read_vmcs(
                    self.instance,
                    hv_sys::VMCS_RO_EXIT_REASON,
                    &mut exit_reason,
                )
            },
            RunError::ReadExitReason
        )?;

        Ok(HfExit {
            cpu: PhantomData,
            exit_reason,
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
    #[cfg(target_arch = "x86_64")]
    rsp: State<usize>,
    #[cfg(target_arch = "x86_64")]
    rip: State<usize>,
    #[cfg(target_arch = "x86_64")]
    cr0: State<usize>,
    #[cfg(target_arch = "x86_64")]
    cr3: State<usize>,
    #[cfg(target_arch = "x86_64")]
    cr4: State<usize>,
    #[cfg(target_arch = "x86_64")]
    cs: State<u64>,
    #[cfg(target_arch = "x86_64")]
    ds: State<usize>,
    #[cfg(target_arch = "x86_64")]
    es: State<usize>,
    #[cfg(target_arch = "x86_64")]
    fs: State<usize>,
    #[cfg(target_arch = "x86_64")]
    gs: State<usize>,
    #[cfg(target_arch = "x86_64")]
    ss: State<usize>,
    #[cfg(target_arch = "aarch64")]
    tcr_el1: State<u64>,
    #[cfg(target_arch = "aarch64")]
    ttbr1_el1: State<u64>,
    #[cfg(target_arch = "aarch64")]
    sp_el1: State<u64>,
}

impl<'a, 'b> CpuStates for HfStates<'a, 'b> {
    type Err = StatesError;

    #[cfg(target_arch = "x86_64")]
    fn set_rdi(&mut self, v: usize) {
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn set_rsp(&mut self, v: usize) {
        self.rsp = State::Dirty(v);
    }

    #[cfg(target_arch = "x86_64")]
    fn set_rip(&mut self, v: usize) {
        self.rip = State::Dirty(v);
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cr0(&mut self, v: usize) {
        self.cr0 = State::Dirty(v);
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cr3(&mut self, v: usize) {
        self.cr3 = State::Dirty(v);
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cr4(&mut self, v: usize) {
        self.cr4 = State::Dirty(v);
    }

    #[cfg(target_arch = "x86_64")]
    fn set_efer(&mut self, v: usize) {
        // LME and LMA bits
        // There seems to be now way to actually set these, however they appear to be set by default
        assert_eq!(v, 0x100 | 0x400);
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

    #[cfg(target_arch = "aarch64")]
    fn set_tcr_el1(&mut self, ips: u8, tg1: u8, a1: bool, t1sz: u8, tg0: u8, t0sz: u8) {
        let ips: u64 = ips.into();
        let tg1: u64 = tg1.into();
        let a1: u64 = a1.into();
        let t1sz: u64 = t1sz.into();
        let tg0: u64 = tg0.into();
        let t0sz: u64 = t0sz.into();

        assert_eq!(ips & 0b11111000, 0);
        assert_eq!(tg1 & 0b11111100, 0);
        assert_eq!(t1sz & 0b11000000, 0);
        assert_eq!(tg0 & 0b11111100, 0);
        assert_eq!(t0sz & 0b11000000, 0);

        self.tcr_el1 =
            State::Dirty(ips << 32 | tg1 << 30 | a1 << 22 | t1sz << 16 | tg0 << 14 | t0sz);
    }

    #[cfg(target_arch = "aarch64")]
    fn set_ttbr1_el1(&mut self, baddr: usize) {
        assert_eq!(baddr & 0xFFFF000000000001, 0);

        self.ttbr1_el1 = State::Dirty(baddr.try_into().unwrap());
    }

    #[cfg(target_arch = "aarch64")]
    fn set_sp_el1(&mut self, v: usize) {
        self.sp_el1 = State::Dirty(v.try_into().unwrap());
    }

    #[cfg(target_arch = "aarch64")]
    fn set_pc(&mut self, v: usize) {
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn commit(self) -> Result<(), Self::Err> {
        if let State::Dirty(v) = self.rip {
            self.cpu
                .write_register(hv_sys::hv_x86_reg_t_HV_X86_RIP, v)
                .map_err(StatesError::SetRipFailed)?;
        }

        if let State::Dirty(v) = self.rsp {
            self.cpu
                .write_register(hv_sys::hv_x86_reg_t_HV_X86_RSP, v)
                .map_err(StatesError::SetRspFailed)?;
        }

        if let State::Dirty(v) = self.cr0 {
            self.cpu
                .write_register(hv_sys::hv_x86_reg_t_HV_X86_CR0, v)
                .map_err(StatesError::SetCr0Failed)?;
        }

        if let State::Dirty(v) = self.cr3 {
            self.cpu
                .write_register(hv_sys::hv_x86_reg_t_HV_X86_CR3, v)
                .map_err(StatesError::SetCr3Failed)?;
        }

        if let State::Dirty(v) = self.cr4 {
            self.cpu
                .write_register(hv_sys::hv_x86_reg_t_HV_X86_CR4, v)
                .map_err(StatesError::SetCr4Failed)?;
        }

        Ok(())
    }

    #[cfg(target_arch = "aarch64")]
    fn commit(self) -> Result<(), Self::Err> {
        use hv_sys::{
            hv_sys_reg_t_HV_SYS_REG_SP_EL1 as HV_SYS_REG_SP_EL1,
            hv_sys_reg_t_HV_SYS_REG_TCR_EL1 as HV_SYS_REG_TCR_EL1,
            hv_sys_reg_t_HV_SYS_REG_TTBR1_EL1 as HV_SYS_REG_TTBR1_EL1, hv_vcpu_set_sys_reg,
        };

        // Set system registers.
        let cpu = self.cpu.instance;
        let set_sys = |reg, val| match NonZero::new(unsafe { hv_vcpu_set_sys_reg(cpu, reg, val) }) {
            Some(v) => Err(v),
            None => Ok(()),
        };

        if let State::Dirty(v) = self.tcr_el1 {
            set_sys(HV_SYS_REG_TCR_EL1, v).map_err(StatesError::SetTcrEl1Failed)?;
        }

        if let State::Dirty(v) = self.ttbr1_el1 {
            set_sys(HV_SYS_REG_TTBR1_EL1, v).map_err(StatesError::SetTtbr1El1Failed)?;
        }

        if let State::Dirty(v) = self.sp_el1 {
            set_sys(HV_SYS_REG_SP_EL1, v).map_err(StatesError::SetSpEl1Failed)?;
        }

        Ok(())
    }
}

enum State<T> {
    None,
    Clean(T),
    Dirty(T),
}

/// Implementation of [`Cpu::Exit`] for Hypervisor Framework.
pub struct HfExit<'a> {
    cpu: PhantomData<&'a mut HfCpu<'a>>,
    #[cfg(target_arch = "x86_64")]
    exit_reason: u64,
}

impl<'a> CpuExit for HfExit<'a> {
    type Io = HfIo;

    #[cfg(target_arch = "x86_64")]
    fn into_hlt(self) -> Result<(), Self> {
        match self.exit_reason.try_into() {
            Ok(hv_sys::VMX_REASON_HLT) => Ok(()),
            _ => Err(self),
        }
    }

    fn into_io(self) -> Result<Self::Io, Self> {
        todo!();
    }
}

/// Implementation of [`CpuIo`] for Hypervisor Framework.
pub struct HfIo {}

impl CpuIo for HfIo {
    fn addr(&self) -> usize {
        todo!();
    }

    fn buffer(&mut self) -> IoBuf {
        todo!();
    }

    fn translate(&self, vaddr: usize) -> Result<usize, Box<dyn Error>> {
        todo!();
    }
}

/// Implementation of [`Cpu::RunErr`].
#[derive(Debug, Error)]
pub enum RunError {
    #[error("error running vcpu ({0:#x})")]
    Run(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading exit reason ({0:#x})")]
    ReadExitReason(NonZero<hv_sys::hv_return_t>),
}

/// Implementation of [`Cpu::GetStatesErr`] and [`CpuStates::Err`].
#[derive(Debug, Error)]
pub enum StatesError {
    #[cfg(target_arch = "x86_64")]
    #[error("couldn't set RIP")]
    SetRipFailed(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "x86_64")]
    #[error("couldn't set RSP")]
    SetRspFailed(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "x86_64")]
    #[error("couldn't set CR0")]
    SetCr0Failed(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "x86_64")]
    #[error("couldn't set CR3")]
    SetCr3Failed(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "x86_64")]
    #[error("couldn't set CR4")]
    SetCr4Failed(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't set TCR_EL1")]
    SetTcrEl1Failed(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't set TTBR1_EL1")]
    SetTtbr1El1Failed(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't set SP_EL1")]
    SetSpEl1Failed(NonZero<hv_sys::hv_return_t>),
}
