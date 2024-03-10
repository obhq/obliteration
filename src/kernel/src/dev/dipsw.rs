use crate::{
    errno::{Errno, EINVAL},
    fs::{Device, IoCmd},
    process::VThread,
};
use macros::Errno;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
struct Dipsw {}

impl Device for Dipsw {
    fn ioctl(self: Arc<Self>, cmd: IoCmd, td: &VThread) -> Result<(), Box<dyn Errno>> {
        if !td.cred().is_system() {
            return Err(Box::new(IoctlErr::InsufficientCredentials));
        }

        match cmd {
            IoCmd::DIPSWINIT => todo!(),
            IoCmd::DIPSWSET(_) => todo!(),
            IoCmd::DIPSWUNSET(_) => todo!(),
            IoCmd::DIPSWCHECK(_) => todo!(),
            IoCmd::DIPSWREAD(_) => todo!(),
            IoCmd::DIPSWWRITE(_) => todo!(),
            IoCmd::DIPSWCHECK2(data) => *data = false as i32,
            _ => todo!(),
        }

        Ok(())
    }
}

#[derive(Error, Debug, Errno)]
enum IoctlErr {
    #[error("bad credentials")]
    #[errno(EINVAL)]
    InsufficientCredentials,
}
