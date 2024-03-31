use crate::{
    errno::Errno,
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
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        let td = td.unwrap();

        if !td.cred().is_system() {
            todo!()
        } else {
            todo!()
        }
    }
}
