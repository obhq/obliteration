use crate::{
    errno::Errno,
    fs::{CharacterDevice, DeviceDriver, IoCmd, UioMut},
    process::VThread,
};
use std::sync::Arc;

#[derive(Debug)]
struct Hid {}

impl DeviceDriver for Hid {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn read(
        &self,
        dev: &Arc<CharacterDevice>,
        data: &mut UioMut,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
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
