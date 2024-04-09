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
struct Gc {}

impl Gc {
    fn new() -> Self {
        Self {}
    }
}

impl DeviceDriver for Gc {
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
        return Ok(());

        match cmd {
            IoCmd::GC12(_) => todo!("GC12 ioctl"),
            IoCmd::GC16(_) => todo!("GC16 ioctl"),
            IoCmd::GC25(_) => todo!("GC25 ioctl"),
            IoCmd::GC27(_) => todo!("GC32 ioctl"),
            IoCmd::GC31(_) => todo!("GC31 ioctl"),
            _ => todo!(),
        }
    }
}

pub struct GcManager {
    gc: Arc<CharacterDevice>,
}

impl GcManager {
    pub fn new() -> Result<Arc<Self>, GcInitError> {
        let gc = make_dev(
            Gc::new(),
            DriverFlags::from_bits_retain(0x80000004),
            0,
            "gc",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o666).unwrap(),
            None,
            MakeDevFlags::MAKEDEV_ETERNAL,
        )?;

        Ok(Arc::new(Self { gc }))
    }
}

/// Represents an error when [`GcManager`] fails to initialize.
#[derive(Debug, Error)]
pub enum GcInitError {
    #[error("cannot create gc device")]
    CreateGcFailed(#[from] MakeDevError),
}
