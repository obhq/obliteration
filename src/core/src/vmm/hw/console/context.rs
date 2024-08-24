use super::Console;
use crate::vmm::hv::{CpuIo, IoBuf};
use crate::vmm::hw::{DeviceContext, Ram};
use crate::vmm::VmmEvent;
use obvirt::console::{Memory, MsgType};
use std::error::Error;
use std::mem::offset_of;
use thiserror::Error;

/// Implementation of [`DeviceContext`].
pub struct Context<'a> {
    dev: &'a Console,
    ram: &'a Ram,
    msg_len: usize,
    msg: String,
}

impl<'a> Context<'a> {
    pub fn new(dev: &'a Console, ram: &'a Ram) -> Self {
        Self {
            dev,
            ram,
            msg_len: 0,
            msg: String::new(),
        }
    }

    fn read_u8(&self, off: usize, exit: &mut dyn CpuIo) -> Result<u8, ExecError> {
        // Get data.
        let data = match exit.buffer() {
            IoBuf::Write(v) => v,
            IoBuf::Read(_) => return Err(ExecError::ReadNotSupported(off)),
        };

        // Parse data.
        if data.len() != 1 {
            Err(ExecError::InvalidData(off))
        } else {
            Ok(data[0])
        }
    }

    fn read_usize(&self, off: usize, exit: &mut dyn CpuIo) -> Result<usize, ExecError> {
        // Get data.
        let data = match exit.buffer() {
            IoBuf::Write(v) => v,
            IoBuf::Read(_) => return Err(ExecError::ReadNotSupported(off)),
        };

        // Parse data.
        data.try_into()
            .map(|v| usize::from_ne_bytes(v))
            .map_err(|_| ExecError::InvalidData(off))
    }

    fn read_str<'b>(
        &self,
        off: usize,
        exit: &'b mut dyn CpuIo,
        len: usize,
    ) -> Result<&'b str, ExecError> {
        // Get data.
        let buf = match exit.buffer() {
            IoBuf::Write(v) => v,
            IoBuf::Read(_) => return Err(ExecError::ReadNotSupported(off)),
        };

        // Get address.
        let vaddr = buf
            .try_into()
            .map(|v| usize::from_ne_bytes(v))
            .map_err(|_| ExecError::InvalidData(off))?;
        let paddr = exit
            .translate(vaddr)
            .map_err(|e| ExecError::TranslateVaddrFailed(vaddr, e))?;

        // Read data.
        let data = unsafe { self.ram.host_addr().add(paddr) };
        let data = unsafe { std::slice::from_raw_parts(data, len) };

        Ok(std::str::from_utf8(data).unwrap())
    }
}

impl<'a> DeviceContext for Context<'a> {
    fn exec(&mut self, exit: &mut dyn CpuIo) -> Result<bool, Box<dyn Error>> {
        // Check field.
        let off = exit.addr() - self.dev.addr;

        if off == offset_of!(Memory, msg_len) {
            self.msg_len = self.read_usize(off, exit)?;
        } else if off == offset_of!(Memory, msg_addr) {
            self.msg.push_str(self.read_str(off, exit, self.msg_len)?);
        } else if off == offset_of!(Memory, commit) {
            // Parse data.
            let commit = self.read_u8(off, exit)?;
            let ty = match MsgType::from_u8(commit) {
                Some(v) => v,
                None => return Err(Box::new(ExecError::InvalidCommit(commit))),
            };

            // Trigger event.
            let msg = std::mem::take(&mut self.msg);
            let status = match ty {
                MsgType::Info => unsafe {
                    self.dev.event.invoke(VmmEvent::Log {
                        ty: ty.into(),
                        data: msg.as_ptr().cast(),
                        len: msg.len(),
                    })
                },
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

    #[error("read at offset {0:#} is not supported")]
    ReadNotSupported(usize),

    #[error("invalid data for offset {0:#}")]
    InvalidData(usize),

    #[error("couldn't translate {0:#x} to physical address")]
    TranslateVaddrFailed(usize, #[source] Box<dyn Error>),

    #[error("{0:#} is not a valid commit")]
    InvalidCommit(u8),
}
