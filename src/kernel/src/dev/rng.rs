use crate::{
    errno::Errno,
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
        _: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        match cmd {
            IoCmd::RNG1 => todo!(),
            IoCmd::RNG2 => todo!(),
            _ => todo!(), // ENOIOCTL,
        }
    }
}
