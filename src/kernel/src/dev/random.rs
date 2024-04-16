use crate::errno::Errno;
use crate::fs::{CharacterDevice, DefaultDeviceError, DeviceDriver, IoCmd, IoLen, IoVec, IoVecMut};
use crate::process::VThread;
use std::sync::Arc;

#[derive(Debug)]
struct Random {}

impl DeviceDriver for Random {
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

    fn ioctl(
        &self,
        _: &Arc<CharacterDevice>,
        cmd: IoCmd,
        _: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        match cmd {
            IoCmd::FIOASYNC(_) | IoCmd::FIONBIO(_) => Ok(()),
            _ => Err(Box::new(DefaultDeviceError::CommandNotSupported)),
        }
    }
}
