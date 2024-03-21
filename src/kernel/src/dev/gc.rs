use crate::{
    errno::Errno,
    fs::{CharacterDevice, Device, IoCmd, OpenFlags},
    process::VThread,
};
use std::sync::Arc;

#[derive(Debug)]
struct Gc {}

impl Device for Gc {
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
    fn ioctl(
        &self,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}
