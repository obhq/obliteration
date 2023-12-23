use crate::errno::{Errno, ENXIO};
use crate::fs::{IoctlCom, VFile, VFileOps, VPath};
use crate::process::{VProc, VThread};
use crate::tty::Tty;
use crate::ucred::Ucred;
use macros::vpath;
use std::fmt::{Display, Formatter};
use std::io::{self, Write};
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

/// An implementation of `/dev/console`.
#[derive(Debug)]
pub struct Console {
    tty: Arc<Tty>,
}

impl Console {
    pub const PATH: &VPath = vpath!("/dev/console");

    pub fn new(vp: &Arc<VProc>) -> Self {
        Self {
            tty: Tty::new(vp.clone()),
        }
    }
}

impl VFileOps for Console {
    fn write(
        &self,
        _file: &VFile,
        data: &[u8],
        _cred: &Ucred,
        _td: &VThread,
    ) -> Result<usize, Box<dyn Errno>> {
        let stderr = io::stderr();
        let mut handle = stderr.lock();

        match handle.write(data) {
            Ok(ret) => Ok(ret),
            Err(e) => todo!("Encountered error {e} while writing to stderr."),
        }
    }

    fn ioctl(
        &self,
        _file: &VFile,
        com: IoctlCom,
        _data: &mut [u8],
        _cred: &Ucred,
        td: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        if self.tty.is_gone() || !self.tty.is_open() {
            return Err(Box::new(IoctlErr::TtyNotAvailable));
        }

        //TODO: implement tty_wait_background and the rest of the checks here.

        self.tty.ioctl(com, _data, td)?;

        Ok(())
    }
}
impl Display for Console {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Self::PATH.fmt(f)
    }
}

#[derive(Debug, Error)]
enum IoctlErr {
    #[error("tty is not available")]
    TtyNotAvailable,
}

impl Errno for IoctlErr {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::TtyNotAvailable => ENXIO,
        }
    }
}
