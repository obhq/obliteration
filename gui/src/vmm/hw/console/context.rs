// SPDX-License-Identifier: MIT OR Apache-2.0
use super::Console;
use crate::vmm::hv::{CpuIo, Hypervisor};
use crate::vmm::hw::{read_bin, read_u8, read_usize, DeviceContext, MmioError};
use crate::vmm::VmmEvent;
use obconf::{ConsoleMemory, ConsoleType};
use std::error::Error;
use std::mem::offset_of;
use thiserror::Error;

/// Implementation of [`DeviceContext`].
pub struct Context<'a, H> {
    dev: &'a Console,
    hv: &'a H,
    msg_len: usize,
    msg: Vec<u8>,
}

impl<'a, H: Hypervisor> Context<'a, H> {
    pub fn new(dev: &'a Console, hv: &'a H) -> Self {
        Self {
            dev,
            hv,
            msg_len: 0,
            msg: Vec::new(),
        }
    }
}

impl<'a, H: Hypervisor> DeviceContext for Context<'a, H> {
    fn exec(&mut self, exit: &mut dyn CpuIo) -> Result<bool, Box<dyn Error>> {
        // Check field.
        let off = exit.addr() - self.dev.addr;

        if off == offset_of!(ConsoleMemory, msg_len) {
            self.msg_len = read_usize(exit).map_err(|e| ExecError::ReadFailed(off, e))?;
        } else if off == offset_of!(ConsoleMemory, msg_addr) {
            let data =
                read_bin(exit, self.msg_len, self.hv).map_err(|e| ExecError::ReadFailed(off, e))?;

            self.msg.extend_from_slice(data);
        } else if off == offset_of!(ConsoleMemory, commit) {
            // Parse data.
            let commit = read_u8(exit).map_err(|e| ExecError::ReadFailed(off, e))?;
            let ty: ConsoleType = commit
                .try_into()
                .map_err(|_| Box::new(ExecError::InvalidCommit(commit)))?;

            // Trigger event.
            let msg = std::mem::take(&mut self.msg);
            let status = unsafe {
                self.dev.event.invoke(VmmEvent::Log {
                    ty: ty.into(),
                    data: msg.as_ptr().cast(),
                    len: msg.len(),
                })
            };

            if !status {
                return Ok(false);
            }
        } else {
            return Err(Box::new(ExecError::UnknownField(off)));
        }

        Ok(true)
    }
}

/// Represents an error when [`Context::exec()`] fails.
#[derive(Debug, Error)]
enum ExecError {
    #[error("unknown field at offset {0:#}")]
    UnknownField(usize),

    #[error("couldn't read data for offset {0:#}")]
    ReadFailed(usize, #[source] MmioError),

    #[error("{0:#} is not a valid commit")]
    InvalidCommit(u8),
}
