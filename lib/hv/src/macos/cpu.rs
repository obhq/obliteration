// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::{
    Cpu, CpuCommit, CpuDebug, CpuExit, CpuIo, CpuRun, CpuStates, DebugEvent, IoBuf, Pstate, Sctlr,
    Tcr,
};
use aarch64::Esr;
use applevisor_sys::hv_exit_reason_t::HV_EXIT_REASON_EXCEPTION;
use applevisor_sys::hv_reg_t::{
    HV_REG_CPSR, HV_REG_PC, HV_REG_X0, HV_REG_X1, HV_REG_X2, HV_REG_X3, HV_REG_X4, HV_REG_X5,
    HV_REG_X6, HV_REG_X7, HV_REG_X8, HV_REG_X9, HV_REG_X10, HV_REG_X11, HV_REG_X12, HV_REG_X13,
    HV_REG_X14, HV_REG_X15, HV_REG_X16, HV_REG_X17, HV_REG_X18, HV_REG_X19, HV_REG_X20, HV_REG_X21,
    HV_REG_X22, HV_REG_X23, HV_REG_X24, HV_REG_X25, HV_REG_X26, HV_REG_X27, HV_REG_X28, HV_REG_X29,
    HV_REG_X30,
};
use applevisor_sys::hv_sys_reg_t::{
    HV_SYS_REG_MAIR_EL1, HV_SYS_REG_SCTLR_EL1, HV_SYS_REG_SP_EL1, HV_SYS_REG_TCR_EL1,
    HV_SYS_REG_TTBR0_EL1, HV_SYS_REG_TTBR1_EL1,
};
use applevisor_sys::{
    hv_reg_t, hv_return_t, hv_vcpu_destroy, hv_vcpu_exit_t, hv_vcpu_get_reg, hv_vcpu_run,
    hv_vcpu_set_reg, hv_vcpu_set_sys_reg, hv_vcpu_t,
};
use std::marker::PhantomData;
use std::num::NonZero;
use thiserror::Error;

/// Implementation of [`Cpu`] for Hypervisor Framework.
pub struct HvfCpu<'a> {
    instance: hv_vcpu_t,
    exit: *const hv_vcpu_exit_t,
    vm: PhantomData<&'a ()>,
}

impl<'a> HvfCpu<'a> {
    pub fn new(instance: hv_vcpu_t, exit: *const hv_vcpu_exit_t) -> Self {
        Self {
            instance,
            exit,
            vm: PhantomData,
        }
    }
}

impl<'a> Drop for HvfCpu<'a> {
    fn drop(&mut self) {
        let ret = unsafe { hv_vcpu_destroy(self.instance) };

        if ret != 0 {
            panic!("hv_vcpu_destroy() failed with {ret:#x}");
        }
    }
}

impl<'a> Cpu for HvfCpu<'a> {
    type States<'b>
        = HvfStates<'b, 'a>
    where
        Self: 'b;
    type GetStatesErr = StatesError;
    type Exit<'b>
        = HvfExit<'b, 'a>
    where
        Self: 'b;
    type TranslateErr = std::io::Error;

    fn id(&self) -> usize {
        todo!()
    }

    fn states(&mut self) -> Result<Self::States<'_>, Self::GetStatesErr> {
        Ok(HvfStates {
            cpu: self,
            pstate: State::None,
            sctlr: State::None,
            mair_el1: State::None,
            tcr: State::None,
            ttbr0_el1: State::None,
            ttbr1_el1: State::None,
            sp_el1: State::None,
            pc: State::None,
            x0: State::None,
            x1: State::None,
            x2: State::None,
        })
    }

    fn translate(&self, vaddr: usize) -> Result<usize, std::io::Error> {
        todo!();
    }
}

impl<'a> CpuRun for HvfCpu<'a> {
    type RunErr = RunError;

    fn run(&mut self) -> Result<Self::Exit<'_>, Self::RunErr> {
        match NonZero::new(unsafe { hv_vcpu_run(self.instance) }) {
            Some(v) => Err(RunError::HypervisorFailed(v)),
            None => Ok(HvfExit(self)),
        }
    }
}

/// Implementation of [`Cpu::States`] for Hypervisor Framework.
pub struct HvfStates<'a, 'b> {
    cpu: &'a mut HvfCpu<'b>,
    pstate: State<u64>,
    sctlr: State<u64>,
    mair_el1: State<u64>,
    tcr: State<u64>,
    ttbr0_el1: State<u64>,
    ttbr1_el1: State<u64>,
    sp_el1: State<u64>,
    pc: State<u64>,
    x0: State<u64>,
    x1: State<u64>,
    x2: State<u64>,
}

impl<'a, 'b> CpuStates for HvfStates<'a, 'b> {
    type Err = StatesError;

    fn set_pstate(&mut self, v: Pstate) {
        self.pstate = State::Dirty(v.into_bits());
    }

    fn set_sctlr(&mut self, v: Sctlr) {
        self.sctlr = State::Dirty(v.into_bits());
    }

    fn set_mair_el1(&mut self, attrs: u64) {
        self.mair_el1 = State::Dirty(attrs);
    }

    fn set_tcr(&mut self, v: Tcr) {
        self.tcr = State::Dirty(v.into_bits());
    }

    fn set_ttbr0_el1(&mut self, baddr: usize) {
        assert_eq!(baddr & 0xFFFF000000000001, 0);

        self.ttbr0_el1 = State::Dirty(baddr.try_into().unwrap());
    }

    fn set_ttbr1_el1(&mut self, baddr: usize) {
        assert_eq!(baddr & 0xFFFF000000000001, 0);

        self.ttbr1_el1 = State::Dirty(baddr.try_into().unwrap());
    }

    fn set_sp_el1(&mut self, v: usize) {
        self.sp_el1 = State::Dirty(v.try_into().unwrap());
    }

    fn set_pc(&mut self, v: usize) {
        self.pc = State::Dirty(v.try_into().unwrap());
    }

    fn set_x0(&mut self, v: usize) {
        self.x0 = State::Dirty(v.try_into().unwrap());
    }

    fn set_x1(&mut self, v: usize) {
        self.x1 = State::Dirty(v.try_into().unwrap());
    }

    fn set_x2(&mut self, v: usize) {
        self.x2 = State::Dirty(v.try_into().unwrap());
    }
}

impl<'a, 'b> CpuCommit for HvfStates<'a, 'b> {
    fn commit(self) -> Result<(), Self::Err> {
        // Set PSTATE. Hypervisor Framework use CPSR to represent PSTATE.
        let cpu = self.cpu.instance;
        let set_reg = |reg, val| match NonZero::new(unsafe { hv_vcpu_set_reg(cpu, reg, val) }) {
            Some(v) => Err(StatesError::SetRegister(reg, v)),
            None => Ok(()),
        };

        if let State::Dirty(v) = self.pstate {
            set_reg(HV_REG_CPSR, v)?;
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

        if let State::Dirty(v) = self.tcr {
            set_sys(HV_SYS_REG_TCR_EL1, v).map_err(StatesError::SetTcrFailed)?;
        }

        if let State::Dirty(v) = self.sctlr {
            set_sys(HV_SYS_REG_SCTLR_EL1, v).map_err(StatesError::SetSctlrFailed)?;
        }

        if let State::Dirty(v) = self.sp_el1 {
            set_sys(HV_SYS_REG_SP_EL1, v).map_err(StatesError::SetSpEl1Failed)?;
        }

        // Set general registers.
        if let State::Dirty(v) = self.pc {
            set_reg(HV_REG_PC, v)?;
        }

        if let State::Dirty(v) = self.x0 {
            set_reg(HV_REG_X0, v)?;
        }

        if let State::Dirty(v) = self.x1 {
            set_reg(HV_REG_X1, v)?;
        }

        if let State::Dirty(v) = self.x2 {
            set_reg(HV_REG_X2, v)?;
        }

        Ok(())
    }
}

enum State<T> {
    None,
    Clean(T),
    Dirty(T),
}

/// Implementation of [`CpuExit`] for Hypervisor Framework.
pub struct HvfExit<'a, 'b>(&'a mut HvfCpu<'b>);

impl<'a, 'b> CpuExit for HvfExit<'a, 'b> {
    type Cpu = HvfCpu<'b>;
    type Io = HvfIo<'a, 'b>;
    type Debug = HvfDebug<'a, 'b>;

    fn cpu(&mut self) -> &mut Self::Cpu {
        self.0
    }

    fn into_io(self) -> Result<Self::Io, Self> {
        // Check reason.
        let e = unsafe { &*self.0.exit };

        if e.reason != HV_EXIT_REASON_EXCEPTION {
            return Err(self);
        }

        // Check if Data Abort exception from a lower Exception level.
        let s = Esr::from_bits(e.exception.syndrome);

        if s.ec() != 0b100100 {
            return Err(self);
        }

        // Set PC to next instruction.
        let mut pc = 0;

        assert_eq!(
            unsafe { hv_vcpu_get_reg(self.0.instance, HV_REG_PC, &mut pc) },
            0
        );

        pc += 4;

        assert_eq!(
            unsafe { hv_vcpu_set_reg(self.0.instance, HV_REG_PC, pc) },
            0
        );

        Ok(HvfIo {
            cpu: self.0,
            esr: s,
            addr: e.exception.physical_address.try_into().unwrap(),
            buf: [0; 8],
        })
    }

    fn into_debug(self) -> Result<Self::Debug, Self> {
        todo!()
    }
}

/// Implementation of [`CpuIo`] for Hypervisor Framework.
pub struct HvfIo<'a, 'b> {
    cpu: &'a mut HvfCpu<'b>,
    esr: Esr,
    addr: usize,
    buf: [u8; 8],
}

impl<'a, 'b> CpuIo for HvfIo<'a, 'b> {
    type Cpu = HvfCpu<'b>;

    fn addr(&self) -> usize {
        self.addr
    }

    fn buffer(&mut self) -> IoBuf<'_> {
        // Check if we have syndrome information.
        let iss = self.esr.iss();
        let isv = (iss >> 24) & 0x1;
        let wnr = (iss >> 6) & 0x1;

        if isv != 0 {
            // Get data length.
            let srt = (iss >> 16) & 0x1F;
            let len = match (iss >> 22) & 0x3 {
                0b00 => 1,
                0b01 => 2,
                0b10 => 4,
                0b11 => 8,
                _ => unreachable!(),
            };

            // Get register.
            let reg = match srt {
                0 => HV_REG_X0,
                1 => HV_REG_X1,
                2 => HV_REG_X2,
                3 => HV_REG_X3,
                4 => HV_REG_X4,
                5 => HV_REG_X5,
                6 => HV_REG_X6,
                7 => HV_REG_X7,
                8 => HV_REG_X8,
                9 => HV_REG_X9,
                10 => HV_REG_X10,
                11 => HV_REG_X11,
                12 => HV_REG_X12,
                13 => HV_REG_X13,
                14 => HV_REG_X14,
                15 => HV_REG_X15,
                16 => HV_REG_X16,
                17 => HV_REG_X17,
                18 => HV_REG_X18,
                19 => HV_REG_X19,
                20 => HV_REG_X20,
                21 => HV_REG_X21,
                22 => HV_REG_X22,
                23 => HV_REG_X23,
                24 => HV_REG_X24,
                25 => HV_REG_X25,
                26 => HV_REG_X26,
                27 => HV_REG_X27,
                28 => HV_REG_X28,
                29 => HV_REG_X29,
                30 => HV_REG_X30,
                _ => unreachable!(),
            };

            // Check if write.
            if wnr != 0 {
                // Read register value.
                let mut v = 0;

                assert_eq!(
                    unsafe { hv_vcpu_get_reg(self.cpu.instance, reg, &mut v) },
                    0
                );

                self.buf = v.to_ne_bytes();

                IoBuf::Write(&self.buf[..len])
            } else {
                todo!()
            }
        } else {
            todo!()
        }
    }

    fn cpu(&mut self) -> &mut Self::Cpu {
        self.cpu
    }
}

/// Implementation of [`CpuDebug`] for Hypervisor Framework.
pub struct HvfDebug<'a, 'b>(&'a mut HvfCpu<'b>);

impl<'a, 'b> CpuDebug for HvfDebug<'a, 'b> {
    type Cpu = HvfCpu<'b>;

    fn reason(&mut self) -> DebugEvent {
        todo!()
    }

    fn cpu(&mut self) -> &mut Self::Cpu {
        todo!()
    }
}

/// Implementation of [`Cpu::RunErr`].
#[derive(Debug, Error)]
pub enum RunError {
    #[error("Hypervisor Framework failed ({0:#x})")]
    HypervisorFailed(NonZero<hv_return_t>),
}

/// Implementation of [`Cpu::GetStatesErr`] and [`CpuStates::Err`].
#[derive(Debug, Error)]
pub enum StatesError {
    #[error("couldn't set SCTLR_EL1")]
    SetSctlrFailed(NonZero<hv_return_t>),

    #[error("couldn't set TCR_EL1")]
    SetTcrFailed(NonZero<hv_return_t>),

    #[error("couldn't set MAIR_EL1")]
    SetMairEl1Failed(NonZero<hv_return_t>),

    #[error("couldn't set TTBR0_EL1")]
    SetTtbr0El1Failed(NonZero<hv_return_t>),

    #[error("couldn't set TTBR1_EL1")]
    SetTtbr1El1Failed(NonZero<hv_return_t>),

    #[error("couldn't set SP_EL1")]
    SetSpEl1Failed(NonZero<hv_return_t>),

    #[error("couldn't set {0:?} (code: {1:#x})")]
    SetRegister(hv_reg_t, NonZero<hv_return_t>),
}
