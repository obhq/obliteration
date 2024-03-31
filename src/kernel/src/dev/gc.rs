use crate::{
    errno::AsErrno,
    fs::{CharacterDevice, DeviceDriver, IoCmd, OpenFlags},
    process::VThread,
};
use std::sync::Arc;

#[derive(Debug)]
struct Gc {}

impl DeviceDriver for Gc {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn open(
        &self,
        dev: &Arc<CharacterDevice>,
        mode: OpenFlags,
        devtype: i32,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn AsErrno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn ioctl(
        &self,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: &VThread,
    ) -> Result<(), Box<dyn AsErrno>> {
        todo!()
    }
}
