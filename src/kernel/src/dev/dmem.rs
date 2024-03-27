use crate::{
    errno::{Errno, EPERM},
    fs::{CharacterDevice, DeviceDriver, IoCmd},
    process::VThread,
};
use macros::Errno;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
struct Dmem {
    total_size: usize, // TODO: Should be 0x13C_000_000
    container: DmemContainer,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DmemContainer {
    Zero,
    One,
    Two,
}

impl DeviceDriver for Dmem {
    fn ioctl(
        &self,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        let cred = td.cred();

        if cred.is_unk1() || cred.is_unk2() {
            return Err(Box::new(IoctlErr::InsufficientCredentials));
        }

        let proc_dmem_container = td.proc().dmem_container();

        if self.container != DmemContainer::Two
            && self.container as usize != *proc_dmem_container
            && !cred.is_system()
        {
            return Err(Box::new(IoctlErr::InsufficientCredentials));
        }

        match cmd {
            IoCmd::DMEM10(size) => *size = self.total_size,
            IoCmd::DMEMGETPRT(_prt) => todo!(),
            IoCmd::DMEMGETAVAIL(_avail) => todo!(),
            IoCmd::DMEMALLOC(_alloc) => todo!(),
            IoCmd::DMEMQUERY(_query) => todo!(),
            _ => todo!(),
        }

        Ok(())
    }
}

#[derive(Error, Debug, Errno)]
enum IoctlErr {
    #[error("bad credentials")]
    #[errno(EPERM)]
    InsufficientCredentials,
}

#[repr(C)]
#[derive(Debug)]
pub struct PrtAperture {
    addr: usize,
    len: usize,
    id: i64,
}

#[repr(C)]
#[derive(Debug)]
pub struct DmemAvailable {
    start_or_phys_out: usize,
    end: usize,
    align: usize,
    size_out: usize,
}

#[repr(C)]
#[derive(Debug)]
pub struct DmemAllocate {
    start_or_phys_out: usize,
    end: usize,
    len: usize,
    align: usize,
    mem_type: i32,
}

#[repr(C)]
#[derive(Debug)]
pub struct DmemQuery {
    dmem_container: i32,
    flags: i32,
    unk: usize,
    phys_addr: usize,
    info_out: usize,
    info_size: usize,
}
