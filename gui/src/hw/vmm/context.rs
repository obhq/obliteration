// SPDX-License-Identifier: MIT OR Apache-2.0
use super::Vmm;
use crate::hw::{DeviceContext, MmioError, read_u8};
use config::{KernelExit, VmmMemory};
use hv::{Cpu, CpuExit, CpuIo};
use std::error::Error;
use std::mem::offset_of;
use thiserror::Error;

/// Implementation of [`DeviceContext`].
pub struct Context<'a> {
    dev: &'a Vmm,
}

impl<'a> Context<'a> {
    pub fn new(dev: &'a Vmm) -> Self {
        Self { dev }
    }
}

impl<C: Cpu> DeviceContext<C> for Context<'_> {
    fn mmio(
        &mut self,
        exit: &mut <C::Exit<'_> as CpuExit>::Io,
    ) -> Result<Option<bool>, Box<dyn Error + Send + Sync>> {
        // Check field.
        let off = exit.addr() - self.dev.addr;

        if off == offset_of!(VmmMemory, shutdown) {
            let exit = read_u8(exit).map_err(|e| ExecError::ReadFailed(off, e))?;
            let exit: KernelExit = exit
                .try_into()
                .map_err(|_| Box::new(ExecError::InvalidExit(exit)))?;

            Ok(Some(exit == KernelExit::Success))
        } else {
            Err(Box::new(ExecError::UnknownField(off)))
        }
    }
}

/// Represents an error when [`Context::exec()`] fails.
#[derive(Debug, Error)]
enum ExecError {
    #[error("unknown field at offset {0:#}")]
    UnknownField(usize),

    #[error("couldn't read data for offset {0:#}")]
    ReadFailed(usize, #[source] MmioError),

    #[error("{0:#} is not a valid exit status")]
    InvalidExit(u8),
}
