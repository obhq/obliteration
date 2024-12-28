// SPDX-License-Identifier: MIT OR Apache-2.0
use super::Vmm;
use crate::hv::{Cpu, CpuExit, CpuIo};
use crate::vmm::channel::MainStream;
use crate::vmm::hw::{read_u8, DeviceContext, MmioError};
use obconf::{KernelExit, VmmMemory};
use std::error::Error;
use std::mem::offset_of;
use thiserror::Error;

/// Implementation of [`DeviceContext`].
pub struct Context<'a> {
    dev: &'a Vmm,
    main: &'a MainStream,
}

impl<'a> Context<'a> {
    pub fn new(dev: &'a Vmm, main: &'a MainStream) -> Self {
        Self { dev, main }
    }
}

impl<C: Cpu> DeviceContext<C> for Context<'_> {
    fn mmio(
        &mut self,
        exit: &mut <C::Exit<'_> as CpuExit>::Io,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        // Check field.
        let off = exit.addr() - self.dev.addr;

        if off == offset_of!(VmmMemory, shutdown) {
            let exit = read_u8(exit).map_err(|e| ExecError::ReadFailed(off, e))?;
            let exit: KernelExit = exit
                .try_into()
                .map_err(|_| Box::new(ExecError::InvalidExit(exit)))?;

            self.main.exit(exit == KernelExit::Success);

            Ok(false)
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
