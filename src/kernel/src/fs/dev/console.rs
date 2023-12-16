use crate::errno::{Errno, ENXIO};
use crate::fs::{IoctlCom, VFile, VFileOps, VPath};
use crate::process::{VProc, VProcGroup, VThread};
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
    tty: Tty,
}

impl Console {
    pub const PATH: &VPath = vpath!("/dev/console");

    pub const TTY_GRP: u8 = b't';
    pub const TIOCSCTTY: IoctlCom = IoctlCom::io(Self::TTY_GRP, 97);
    pub const TIOCSTI: IoctlCom = IoctlCom::iow::<u8>(Self::TTY_GRP, 114);

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
        _td: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        if self.tty.is_gone() || !self.tty.is_open() {
            return Err(Box::new(IoctlErr::TtyNotAvailable));
        }

        match com {
            Self::TIOCSCTTY => {
                todo!();
            }
            _ => todo!("Unimplemented console ioctl command: {com:?}"),
        }

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

#[derive(Debug)]
pub struct Tty {
    vp: Arc<VProc>,
    group: Arc<Option<VProcGroup>>,
}

impl Tty {
    pub fn new(vp: Arc<VProc>) -> Self {
        Self {
            vp,
            group: Arc::new(None),
        }
    }

    //TODO: implement this
    pub fn is_gone(&self) -> bool {
        false
    }

    //TODO: implement this
    pub fn is_open(&self) -> bool {
        true
    }

    pub fn ioctl(
        &self,
        _com: IoctlCom,
        _data: &mut [u8],
        _td: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        let grp = self.vp.group().expect("Process group is not set.");
        let session = grp.session();

        Ok(())
    }
}
