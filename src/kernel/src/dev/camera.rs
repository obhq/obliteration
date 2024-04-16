use crate::errno::Errno;
use crate::fs::{
    make_dev, CharacterDevice, DeviceDriver, DriverFlags, IoCmd, IoLen, IoVec, IoVecMut,
    MakeDevError, MakeDevFlags, Mode, OpenFlags,
};
use crate::process::VThread;
use crate::ucred::{Gid, Uid};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
struct Camera {}

impl Camera {
    fn new() -> Self {
        Self {}
    }
}

impl DeviceDriver for Camera {
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

    #[allow(unused_variables)] // TODO: remove when implementing
    fn read(
        &self,
        dev: &Arc<CharacterDevice>,
        off: Option<u64>,
        buf: &mut [IoVecMut],
        td: Option<&VThread>,
    ) -> Result<IoLen, Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn write(
        &self,
        dev: &Arc<CharacterDevice>,
        off: Option<u64>,
        buf: &[IoVec],
        td: Option<&VThread>,
    ) -> Result<IoLen, Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn ioctl(
        &self,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}

pub struct CameraManager {
    camera: Arc<CharacterDevice>,
}

impl CameraManager {
    pub fn new() -> Result<Arc<Self>, CameraInitError> {
        let camera = make_dev(
            Camera::new(),
            DriverFlags::from_bits_retain(0x80000004),
            0,
            "camera",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o666).unwrap(),
            None,
            MakeDevFlags::MAKEDEV_ETERNAL,
        )?;

        Ok(Arc::new(Self { camera }))
    }
}

/// Represents an error when [`CameraManager`] fails to initialize.
#[derive(Debug, Error)]
pub enum CameraInitError {
    #[error("cannot create camera device")]
    CreateGcFailed(#[from] MakeDevError),
}
