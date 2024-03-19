use crate::{
    errno::Errno,
    fs::{CharacterDevice, Device, IoCmd},
    process::VThread,
};
use std::sync::Arc;

#[derive(Debug)]
struct Dipsw {}

impl Device for Dipsw {
    #[allow(unused_variables)]
    fn ioctl(
        self: &Arc<Self>,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        if !td.cred().is_system() {
            todo!()
        } else {
            todo!()
        }
    }
}
