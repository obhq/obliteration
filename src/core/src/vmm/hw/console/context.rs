use super::{Console, Log};
use crate::vmm::hv::{CpuIo, IoBuf};
use crate::vmm::hw::{DeviceContext, Ram};
use obvirt::console::{Commit, Memory};
use std::error::Error;
use std::mem::offset_of;
use thiserror::Error;

/// Implementation of [`DeviceContext`].
pub struct Context<'a> {
    dev: &'a Console,
    ram: &'a Ram,
    file_len: usize,
    file: String,
    msg_len: usize,
    msg: String,
}

impl<'a> Context<'a> {
    pub fn new(dev: &'a Console, ram: &'a Ram) -> Self {
        Self {
            dev,
            ram,
            file_len: 0,
            file: String::new(),
            msg_len: 0,
            msg: String::new(),
        }
    }

    fn read_u32(&self, off: usize, exit: &mut dyn CpuIo) -> Result<u32, ExecError> {
        // Get data.
        let data = match exit.buffer() {
            IoBuf::Write(v) => v,
            IoBuf::Read(_) => return Err(ExecError::ReadNotSupported(off)),
        };

        // Parse data.
        data.try_into()
            .map(|v| u32::from_ne_bytes(v))
            .map_err(|_| ExecError::InvalidData(off))
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
    fn exec(&mut self, exit: &mut dyn CpuIo) -> Result<(), Box<dyn Error>> {
        // Check field.
        let off = exit.addr() - self.dev.addr;

        if off == offset_of!(Memory, msg_len) {
            self.msg_len = self.read_usize(off, exit)?;
        } else if off == offset_of!(Memory, msg_addr) {
            self.msg.push_str(self.read_str(off, exit, self.msg_len)?);
        } else if off == offset_of!(Memory, file_len) {
            self.file_len = self.read_usize(off, exit)?;
        } else if off == offset_of!(Memory, file_addr) {
            self.file = self.read_str(off, exit, self.file_len)?.to_owned();
        } else if off == offset_of!(Memory, commit) {
            // Parse data.
            let commit = self.read_u32(off, exit)?;
            let (ty, line) = match Commit::parse(commit) {
                Some(v) => v,
                None => return Err(Box::new(ExecError::InvalidCommit(commit))),
            };

            // Push log.
            let mut logs = self.dev.logs.lock().unwrap();

            logs.push_back(Log {
                ty,
                file: std::mem::take(&mut self.file),
                line,
                msg: std::mem::take(&mut self.msg),
            });

            while logs.len() > 10000 {
                logs.pop_front();
            }
        } else {
            return Err(Box::new(ExecError::UnknownField(off)));
        }

        Ok(())
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
    InvalidCommit(u32),
}
