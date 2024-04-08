use crate::errno::Errno;
use crate::fs::{CharacterDevice, DeviceDriver, IoCmd, IoLen, IoVecMut};
use crate::process::VThread;
use std::sync::Arc;

#[derive(Debug)]
struct Hid {}

impl DeviceDriver for Hid {
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
    fn ioctl(
        &self,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}
