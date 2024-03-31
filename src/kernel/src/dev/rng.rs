use crate::{
    errno::AsErrno,
    fs::{CharacterDevice, DeviceDriver, IoCmd},
    process::VThread,
};
use std::sync::Arc;

#[derive(Debug)]
struct Rng {}

impl DeviceDriver for Rng {
    fn ioctl(
        &self,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        _: &VThread,
    ) -> Result<(), Box<dyn AsErrno>> {
        match cmd {
            IoCmd::RNG1 => todo!(),
            IoCmd::RNG2 => todo!(),
            _ => todo!(), // ENOIOCTL,
        }
    }
}
