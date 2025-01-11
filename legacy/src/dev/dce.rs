use crate::{
    errno::Errno,
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
            IoCmd::DCEREGBUFPTRS(_) => todo!("DCEREGBUFPOINTERS ioctl"),
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

#[repr(C)]
#[derive(Debug)]
pub struct DceFlipControlArg {
    id: u32,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: u64,
    arg5: u64,
}

#[repr(C)]
#[derive(Debug)]
pub struct DceSubmitFlipArg {
    canary: usize,
    buffer_index: usize,
    flip_mode: u32,
    arg1: usize,
    arg2: usize,
    eop_nz: u32,
    eop_val: usize,
    unk: usize,
    rout: usize,
}

#[repr(C)]
#[derive(Debug)]
pub struct DceRegisterBufferPtrsArg {
    canary: usize,
    index: u32,
    attrid: u32,
    left: usize,
    right: usize,
    unk: u32,
    _align: u64,
}

/// Represents an error when [`DceManager`] fails to initialize.
#[derive(Debug, Error)]
pub enum DceInitError {
    #[error("cannot create dce device")]
    CreateDceFailed(#[from] MakeDevError),
}
