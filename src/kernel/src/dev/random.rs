use crate::{
    errno::Errno,
    fs::{CharacterDevice, DefaultDeviceError, DeviceDriver, IoCmd, Uio, UioMut},
    process::VThread,
};
use std::sync::Arc;

#[derive(Debug)]
struct Random {}

impl DeviceDriver for Random {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn read(
        &self,
        dev: &Arc<CharacterDevice>,
        uio: &mut UioMut,
        off: i64,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn write(
        &self,
        dev: &Arc<CharacterDevice>,
        uio: &mut Uio,
        off: i64,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        todo!()
    }

    fn ioctl(
        &self,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        _: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        match cmd {
            IoCmd::FIOASYNC(_) | IoCmd::FIONBIO(_) => Ok(()),
            _ => Err(Box::new(DefaultDeviceError::CommandNotSupported)),
        }
    }
}
