use crate::{
    errno::Errno,
    error,
    fs::{
        make_dev, CharacterDevice, DeviceDriver, DriverFlags, IoCmd, MakeDevError, MakeDevFlags,
        Mode, OpenFlags,
    },
    process::VThread,
    ucred::{Gid, Uid},
};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
struct Dce {}

impl Dce {
    fn new() -> Self {
        Self {}
    }
}

impl DeviceDriver for Dce {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn open(
        &self,
        dev: &Arc<CharacterDevice>,
        mode: OpenFlags,
        devtype: i32,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }

    fn ioctl(
        &self,
        _: &Arc<CharacterDevice>,
        cmd: IoCmd,
        _: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        match cmd {
            IoCmd::DCEFLIPCONTROL(_) => todo!("DCEFLIPCONTROL ioctl"),
            IoCmd::DCESUBMITFLIP(_) => todo!("DCESUBMITFLIP ioctl"),
            IoCmd::DCEREGBUFPOINTERS(_) => todo!("DCEREGBUFPOINTERS ioctl"),
            IoCmd::DCEREGBUFATTR(_) => todo!("DCEREGBUFATTR ioctl"),
            IoCmd::DCEDEREGIDENT(_) => todo!("DCEDEREGIDENT ioctl"),
            _ => todo!(),
        }
    }
}

pub struct DceManager {
    dce: Arc<CharacterDevice>,
}

impl DceManager {
    pub fn new() -> Result<Arc<Self>, DceInitError> {
        let dce = make_dev(
            Dce::new(),
            DriverFlags::INIT,
            0,
            "dce",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o666).unwrap(),
            None,
            MakeDevFlags::empty(),
        )?;

        Ok(Arc::new(Self { dce }))
    }
}

#[derive(Debug)]
pub struct DceFlipControl {
    id: u32,
    spare: u32,
    arg2: usize,
    ptr: usize,
    size: usize,
    unk1: u64,
    unk2: u64,
}

/// Represents an error when [`DceManager`] fails to initialize.
#[derive(Debug, Error)]
pub enum DceInitError {
    #[error("cannot create dce device")]
    CreateDceFailed(#[from] MakeDevError),
}
