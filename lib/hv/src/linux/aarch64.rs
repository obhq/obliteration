// SPDX-License-Identifier: MIT OR Apache-2.0
use super::ffi::{ARM64_SYS_REG, KVM_SET_ONE_REG, KvmOneReg};
use crate::{CpuStates, HypervisorExt, Pstate, Sctlr, Tcr};
use libc::ioctl;
use std::io::Error;
use std::os::fd::{AsRawFd, OwnedFd};
use thiserror::Error;

impl HypervisorExt for Kvm {}

/// Implementation of [`Cpu::States`] for KVM.
pub struct KvmStates<'a> {
    cpu: &'a mut OwnedFd,
    pstate: State<u64>,
    sctlr: State<u64>,
    mair: State<u64>,
    tcr: State<u64>,
    ttbr0: State<u64>,
    ttbr1: State<u64>,
    sp: State<u64>,
    pc: State<u64>,
    x0: State<u64>,
    x1: State<u64>,
}

impl<'a> KvmStates<'a> {
    pub fn from_cpu(cpu: &'a mut OwnedFd) -> Result<Self, StatesError> {
        Ok(KvmStates {
            cpu,
            pstate: State::None,
            sctlr: State::None,
            mair: State::None,
            tcr: State::None,
            ttbr0: State::None,
            ttbr1: State::None,
            sp: State::None,
            pc: State::None,
            x0: State::None,
            x1: State::None,
        })
    }

    fn set_reg<T>(&mut self, reg: u64, mut val: T) -> Result<(), Error> {
        let reg = KvmOneReg {
            id: reg,
            addr: &mut val,
        };

        match unsafe { ioctl(self.cpu.as_raw_fd(), KVM_SET_ONE_REG, &reg) } {
            0 => Ok(()),
            _ => Err(Error::last_os_error()),
        }
    }
}

impl<'a> CpuStates for KvmStates<'a> {
    type Err = StatesError;

    fn set_pstate(&mut self, v: Pstate) {
        self.pstate = State::Dirty(v.into_bits());
    }

    fn set_sctlr(&mut self, v: Sctlr) {
        self.sctlr = State::Dirty(v.into_bits());
    }

    fn set_mair_el1(&mut self, attrs: u64) {
        self.mair = State::Dirty(attrs);
    }

    fn set_tcr(&mut self, v: Tcr) {
        self.tcr = State::Dirty(v.into_bits());
    }

    fn set_ttbr0_el1(&mut self, baddr: usize) {
        self.ttbr0 = State::Dirty(baddr.try_into().unwrap());
    }

    fn set_ttbr1_el1(&mut self, baddr: usize) {
        self.ttbr1 = State::Dirty(baddr.try_into().unwrap());
    }

    fn set_sp_el1(&mut self, v: usize) {
        self.sp = State::Dirty(v.try_into().unwrap());
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

    fn commit(mut self) -> Result<(), Self::Err> {
        // PSTATE.
        if let State::Dirty(v) = self.pstate {
            self.set_reg(0x6030000000100042, v)
                .map_err(StatesError::SetPstateFailed)?;
        }

        // SCTLR_EL1.
        if let State::Dirty(v) = self.sctlr {
            self.set_reg(ARM64_SYS_REG(0b11, 0b000, 0b0001, 0b0000, 0b000), v)
                .map_err(StatesError::SetSctlrFailed)?;
        }

        // MAIR_EL1.
        if let State::Dirty(v) = self.mair {
            self.set_reg(ARM64_SYS_REG(0b11, 0b000, 0b1010, 0b0010, 0b000), v)
                .map_err(StatesError::SetMairFailed)?;
        }

        // TCR_EL1.
        if let State::Dirty(v) = self.tcr {
            self.set_reg(ARM64_SYS_REG(0b11, 0b000, 0b0010, 0b0000, 0b010), v)
                .map_err(StatesError::SetTcrFailed)?;
        }

        // TTBR0_EL1.
        if let State::Dirty(v) = self.ttbr0 {
            self.set_reg(ARM64_SYS_REG(0b11, 0b000, 0b0010, 0b0000, 0b000), v)
                .map_err(StatesError::SetTtbr0Failed)?;
        }

        // TTBR1_EL1.
        if let State::Dirty(v) = self.ttbr1 {
            self.set_reg(ARM64_SYS_REG(0b11, 0b000, 0b0010, 0b0000, 0b001), v)
                .map_err(StatesError::SetTtbr1Failed)?;
        }

        // SP_EL1.
        if let State::Dirty(v) = self.sp {
            self.set_reg(0x6030000000100044, v)
                .map_err(StatesError::SetSpFailed)?;
        }

        // PC.
        if let State::Dirty(v) = self.pc {
            self.set_reg(0x6030000000100040, v)
                .map_err(StatesError::SetPcFailed)?;
        }

        // X0.
        if let State::Dirty(v) = self.x0 {
            self.set_reg(0x6030000000100000, v)
                .map_err(StatesError::SetX0Failed)?;
        }

        // X1.
        if let State::Dirty(v) = self.x1 {
            self.set_reg(0x6030000000100002, v)
                .map_err(StatesError::SetX1Failed)?;
        }

        Ok(())
    }
}

enum State<T> {
    None,
    Dirty(T),
}

/// Implementation of [`CpuStates::Err`].
#[derive(Debug, Error)]
pub enum StatesError {
    #[error("couldn't set PSTATE")]
    SetPstateFailed(#[source] Error),

    #[error("couldn't set SCTLR_EL1")]
    SetSctlrFailed(#[source] Error),

    #[error("couldn't set MAIR_EL1")]
    SetMairFailed(#[source] Error),

    #[error("couldn't set TCR_EL1")]
    SetTcrFailed(#[source] Error),

    #[error("couldn't set TTBR0_EL1")]
    SetTtbr0Failed(#[source] Error),

    #[error("couldn't set TTBR1_EL1")]
    SetTtbr1Failed(#[source] Error),

    #[error("couldn't set SP_EL1")]
    SetSpFailed(#[source] Error),

    #[error("couldn't set PC")]
    SetPcFailed(#[source] Error),

    #[error("couldn't set X0")]
    SetX0Failed(#[source] Error),

    #[error("couldn't set X1")]
    SetX1Failed(#[source] Error),
}
