use crate::{
    errno::{Errno, EPERM},
    fs::{Device, IoCmd},
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
enum DmemContainer {
    Zero,
    One,
    Two,
}

impl Device for Dmem {
    fn ioctl(self: Arc<Self>, cmd: IoCmd, td: &VThread) -> Result<(), Box<dyn Errno>> {
        let cred = td.cred();

        if cred.is_unk1() || cred.is_unk2() {
            return Err(Box::new(IoctlErr::InsufficientCredentials));
        }

        if self.container != DmemContainer::Two
            && self.container as usize != td.proc().dmem_container()
            && !cred.is_system()
        {
            return Err(Box::new(IoctlErr::InsufficientCredentials));
        }

        match cmd {
            IoCmd::DMEM10(size) => *size = self.total_size,
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
