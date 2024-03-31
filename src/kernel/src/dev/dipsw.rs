use crate::{
    errno::AsErrno,
    fs::{CharacterDevice, DeviceDriver, IoCmd},
    process::VThread,
};
use std::sync::Arc;

#[derive(Debug)]
struct Dipsw {}

impl DeviceDriver for Dipsw {
    #[allow(unused_variables)]
    fn ioctl(
        &self,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: &VThread,
    ) -> Result<(), Box<dyn AsErrno>> {
        if !td.cred().is_system() {
            todo!()
        } else {
            todo!()
        }
    }
}
