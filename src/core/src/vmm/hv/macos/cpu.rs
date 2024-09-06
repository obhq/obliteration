// SPDX-License-Identifier: MIT OR Apache-2.0
use super::arch::HfExit;
use crate::vmm::hv::{Cpu, CpuStates};
use hv_sys::hv_vcpu_destroy;
use std::marker::PhantomData;
use std::num::NonZero;
use thiserror::Error;

#[cfg(target_arch = "aarch64")]
#[allow(non_camel_case_types)]
type hv_vcpu_t = hv_sys::hv_vcpu_t;

#[cfg(target_arch = "x86_64")]
#[allow(non_camel_case_types)]
type hv_vcpu_t = hv_sys::hv_vcpuid_t;

#[cfg(target_arch = "x86_64")]
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

    #[cfg(target_arch = "aarch64")]
    pub fn read_sys(&self, reg: hv_sys::hv_sys_reg_t) -> Result<u64, NonZero<hv_sys::hv_return_t>> {
        use hv_sys::hv_vcpu_get_sys_reg;

        let mut v = 0;

        match NonZero::new(unsafe { hv_vcpu_get_sys_reg(self.instance, reg, &mut v) }) {
            Some(v) => Err(v),
            None => Ok(v),
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn read_register(
        &self,
        register: hv_sys::hv_x86_reg_t,
    ) -> Result<usize, NonZero<hv_sys::hv_return_t>> {
        let mut value = std::mem::MaybeUninit::<usize>::uninit();

        wrap_return!(unsafe {
            hv_sys::hv_vcpu_read_register(self.instance, register, value.as_mut_ptr().cast())
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
    type Exit<'b> = HfExit<'b, 'a> where Self: 'b;
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
            pstate: State::None,
            sctlr_el1: State::None,
            mair_el1: State::None,
            tcr_el1: State::None,
            ttbr0_el1: State::None,
            ttbr1_el1: State::None,
            sp_el1: State::None,
            pc: State::None,
            x0: State::None,
        })
    }

    #[cfg(target_arch = "x86_64")]
    fn run(&mut self) -> Result<Self::Exit<'_>, Self::RunErr> {
        wrap_return!(
            unsafe { hv_sys::hv_vcpu_run_until(self.instance, hv_sys::HV_DEADLINE_FOREVER) },
            RunError::HypervisorFailed
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
            RunError::ReadExitFailed
        )?;

        Ok(HfExit::new(exit_reason))
    }

    #[cfg(target_arch = "aarch64")]
    fn run(&mut self) -> Result<Self::Exit<'_>, Self::RunErr> {
        use hv_sys::hv_vcpu_run;

        match NonZero::new(unsafe { hv_vcpu_run(self.instance) }) {
            Some(v) => Err(RunError::HypervisorFailed(v)),
            None => Ok(HfExit::new(self)),
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
    pstate: State<u64>,
    #[cfg(target_arch = "aarch64")]
    sctlr_el1: State<u64>,
    #[cfg(target_arch = "aarch64")]
    mair_el1: State<u64>,
    #[cfg(target_arch = "aarch64")]
    tcr_el1: State<u64>,
    #[cfg(target_arch = "aarch64")]
    ttbr0_el1: State<u64>,
    #[cfg(target_arch = "aarch64")]
    ttbr1_el1: State<u64>,
    #[cfg(target_arch = "aarch64")]
    sp_el1: State<u64>,
    #[cfg(target_arch = "aarch64")]
    pc: State<u64>,
    #[cfg(target_arch = "aarch64")]
    x0: State<u64>,
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
    fn set_pstate(&mut self, v: crate::vmm::hv::Pstate) {
        self.pstate = State::Dirty(v.into_bits().into());
    }

    #[cfg(target_arch = "aarch64")]
    fn set_sctlr_el1(&mut self, m: bool) {
        // All hard-coded values came from https://github.com/AsahiLinux/m1n1/issues/97/
        let m: u64 = m.into();

        self.sctlr_el1 = State::Dirty(0x30901084 | m);
    }

    #[cfg(target_arch = "aarch64")]
    fn set_mair_el1(&mut self, attrs: u64) {
        self.mair_el1 = State::Dirty(attrs);
    }

    #[cfg(target_arch = "aarch64")]
    fn set_tcr_el1(
        &mut self,
        tbi1: bool,
        tbi0: bool,
        ips: u8,
        tg1: u8,
        a1: bool,
        t1sz: u8,
        tg0: u8,
        t0sz: u8,
    ) {
        let tbi1: u64 = tbi1.into();
        let tbi0: u64 = tbi0.into();
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

        self.tcr_el1 = State::Dirty(
            tbi1 << 38
                | tbi0 << 37
                | ips << 32
                | tg1 << 30
                | a1 << 22
                | t1sz << 16
                | tg0 << 14
                | t0sz,
        );
    }

    #[cfg(target_arch = "aarch64")]
    fn set_ttbr0_el1(&mut self, baddr: usize) {
        assert_eq!(baddr & 0xFFFF000000000001, 0);

        self.ttbr0_el1 = State::Dirty(baddr.try_into().unwrap());
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
        self.pc = State::Dirty(v.try_into().unwrap());
    }

    #[cfg(target_arch = "aarch64")]
    fn set_x0(&mut self, v: usize) {
        self.x0 = State::Dirty(v.try_into().unwrap());
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
            hv_reg_t_HV_REG_CPSR as HV_REG_CPSR, hv_reg_t_HV_REG_PC as HV_REG_PC,
            hv_reg_t_HV_REG_X0 as HV_REG_X0,
            hv_sys_reg_t_HV_SYS_REG_MAIR_EL1 as HV_SYS_REG_MAIR_EL1,
            hv_sys_reg_t_HV_SYS_REG_SCTLR_EL1 as HV_SYS_REG_SCTLR_EL1,
            hv_sys_reg_t_HV_SYS_REG_SP_EL1 as HV_SYS_REG_SP_EL1,
            hv_sys_reg_t_HV_SYS_REG_TCR_EL1 as HV_SYS_REG_TCR_EL1,
            hv_sys_reg_t_HV_SYS_REG_TTBR0_EL1 as HV_SYS_REG_TTBR0_EL1,
            hv_sys_reg_t_HV_SYS_REG_TTBR1_EL1 as HV_SYS_REG_TTBR1_EL1, hv_vcpu_set_reg,
            hv_vcpu_set_sys_reg,
        };

        // Set PSTATE. Hypervisor Framework use CPSR to represent PSTATE.
        let cpu = self.cpu.instance;
        let set_reg = |reg, val| match NonZero::new(unsafe { hv_vcpu_set_reg(cpu, reg, val) }) {
            Some(v) => Err(v),
            None => Ok(()),
        };

        if let State::Dirty(v) = self.pstate {
            set_reg(HV_REG_CPSR, v).map_err(StatesError::SetPstateFailed)?;
        }

        // Set system registers.
        let set_sys = |reg, val| match NonZero::new(unsafe { hv_vcpu_set_sys_reg(cpu, reg, val) }) {
            Some(v) => Err(v),
            None => Ok(()),
        };

        if let State::Dirty(v) = self.mair_el1 {
            set_sys(HV_SYS_REG_MAIR_EL1, v).map_err(StatesError::SetMairEl1Failed)?;
        }

        if let State::Dirty(v) = self.ttbr0_el1 {
            set_sys(HV_SYS_REG_TTBR0_EL1, v).map_err(StatesError::SetTtbr0El1Failed)?;
        }

        if let State::Dirty(v) = self.ttbr1_el1 {
            set_sys(HV_SYS_REG_TTBR1_EL1, v).map_err(StatesError::SetTtbr1El1Failed)?;
        }

        if let State::Dirty(v) = self.tcr_el1 {
            set_sys(HV_SYS_REG_TCR_EL1, v).map_err(StatesError::SetTcrEl1Failed)?;
        }

        if let State::Dirty(v) = self.sctlr_el1 {
            set_sys(HV_SYS_REG_SCTLR_EL1, v).map_err(StatesError::SetSctlrEl1Failed)?;
        }

        if let State::Dirty(v) = self.sp_el1 {
            set_sys(HV_SYS_REG_SP_EL1, v).map_err(StatesError::SetSpEl1Failed)?;
        }

        // Set general registers.
        if let State::Dirty(v) = self.pc {
            set_reg(HV_REG_PC, v).map_err(StatesError::SetPcFailed)?;
        }

        if let State::Dirty(v) = self.x0 {
            set_reg(HV_REG_X0, v).map_err(StatesError::SetX0Failed)?;
        }

        Ok(())
    }
}

enum State<T> {
    None,
    Clean(T),
    Dirty(T),
}

/// Implementation of [`Cpu::RunErr`].
#[derive(Debug, Error)]
pub enum RunError {
    #[error("Hypervisor Framework failed ({0:#x})")]
    HypervisorFailed(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "x86_64")]
    #[error("error while reading exit reason ({0:#x})")]
    ReadExitFailed(NonZero<hv_sys::hv_return_t>),
}

/// Implementation of [`Cpu::GetStatesErr`] and [`CpuStates::Err`].
#[derive(Debug, Error)]
pub enum StatesError {
    #[cfg(target_arch = "aarch64")]
    #[error("couldn't read the register")]
    ReadRegisterFailed(NonZero<hv_sys::hv_return_t>),

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
    #[error("couldn't set PSTATE")]
    SetPstateFailed(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't set SCTLR_EL1")]
    SetSctlrEl1Failed(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't set TCR_EL1")]
    SetTcrEl1Failed(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't set MAIR_EL1")]
    SetMairEl1Failed(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't set TTBR0_EL1")]
    SetTtbr0El1Failed(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't set TTBR1_EL1")]
    SetTtbr1El1Failed(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't set SP_EL1")]
    SetSpEl1Failed(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't set PC")]
    SetPcFailed(NonZero<hv_sys::hv_return_t>),

    #[cfg(target_arch = "aarch64")]
    #[error("couldn't set X0")]
    SetX0Failed(NonZero<hv_sys::hv_return_t>),
}
