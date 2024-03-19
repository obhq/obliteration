use crate::{
    errno::Errno,
    fs::{CharacterDevice, Device, IoCmd},
    process::VThread,
};
use std::sync::Arc;

#[derive(Debug)]
struct Gc {}

impl Device for Gc {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn ioctl(
        self: &Arc<Self>,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}
