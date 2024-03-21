use crate::{
    errno::Errno,
    fs::{CharacterDevice, Device, IoCmd},
    process::VThread,
};
use std::sync::Arc;

#[derive(Debug)]
struct SblSrv {}

impl Device for SblSrv {
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
