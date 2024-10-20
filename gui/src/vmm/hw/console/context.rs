// SPDX-License-Identifier: MIT OR Apache-2.0
use super::Console;
use crate::vmm::hv::{Cpu, CpuExit, CpuIo, Hypervisor};
use crate::vmm::hw::{read_ptr, read_u8, read_usize, DeviceContext, MmioError};
use crate::vmm::VmmEvent;
use obconf::{ConsoleMemory, ConsoleType};
use std::error::Error;
use std::mem::offset_of;
use std::num::NonZero;
use thiserror::Error;

/// Implementation of [`DeviceContext`].
pub struct Context<'a, H> {
    dev: &'a Console,
    hv: &'a H,
    msg_len: Option<NonZero<usize>>,
    msg: Vec<u8>,
}

impl<'a, H: Hypervisor> Context<'a, H> {
    pub fn new(dev: &'a Console, hv: &'a H) -> Self {
        Self {
            dev,
            hv,
            msg_len: None,
            msg: Vec::new(),
        }
    }
}

impl<'a, H: Hypervisor, C: Cpu> DeviceContext<C> for Context<'a, H> {
    fn mmio(&mut self, exit: &mut <C::Exit<'_> as CpuExit>::Io) -> Result<bool, Box<dyn Error>> {
        // Check field.
        let off = exit.addr() - self.dev.addr;

        if off == offset_of!(ConsoleMemory, msg_len) {
            self.msg_len = read_usize(exit)
                .map_err(|e| ExecError::ReadFailed(off, e))
                .and_then(|v| NonZero::new(v).ok_or(ExecError::InvalidLen))
                .map(Some)?;
        } else if off == offset_of!(ConsoleMemory, msg_addr) {
            // We don't need to check if length is too large here. The read_ptr will return only
            // allocated memory, which prevent invalid length automatically.
            let len = self.msg_len.take().ok_or(ExecError::InvalidSequence)?;
            let data = read_ptr(exit, len, self.hv).map_err(|e| ExecError::ReadFailed(off, e))?;

            self.msg
                .extend_from_slice(unsafe { std::slice::from_raw_parts(data.as_ptr(), len.get()) });
        } else if off == offset_of!(ConsoleMemory, commit) {
            // Check if state valid.
            if self.msg_len.is_some() || self.msg.is_empty() {
                return Err(Box::new(ExecError::InvalidSequence));
            } else if std::str::from_utf8(&self.msg).is_err() {
                return Err(Box::new(ExecError::InvalidMsg));
            }

            // Parse data.
            let commit = read_u8(exit).map_err(|e| ExecError::ReadFailed(off, e))?;
            let ty: ConsoleType = commit
                .try_into()
                .map_err(|_| Box::new(ExecError::InvalidCommit(commit)))?;

            // Trigger event.
            let msg = std::mem::take(&mut self.msg);

            unsafe {
                self.dev.event.invoke(VmmEvent::Log {
                    ty: ty.into(),
                    data: msg.as_ptr().cast(),
                    len: msg.len(),
                })
            };
        } else {
            return Err(Box::new(ExecError::UnknownField(off)));
        }

        Ok(true)
    }
}

/// Represents an error when [`Context::mmio()`] fails.
#[derive(Debug, Error)]
enum ExecError {
    #[error("unknown field at offset {0:#x}")]
    UnknownField(usize),

    #[error("couldn't read data for offset {0:#x}")]
    ReadFailed(usize, #[source] MmioError),

    #[error("invalid message length")]
    InvalidLen,

    #[error("invalid message")]
    InvalidMsg,

    #[error("{0:#x} is not a valid commit")]
    InvalidCommit(u8),

    #[error("invalid operation sequence")]
    InvalidSequence,
}
