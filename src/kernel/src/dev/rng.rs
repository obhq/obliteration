use crate::{
    errno::Errno,
    fs::{CharacterDevice, Device, IoCmd},
    process::VThread,
};
use std::sync::Arc;

#[derive(Debug)]
struct Rng {}

impl Device for Rng {
    fn ioctl(
        self: &Arc<Self>,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        _: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        match cmd {
            IoCmd::RNG1 => todo!(),
            IoCmd::RNG2 => todo!(),
            _ => todo!(), // ENOIOCTL,
        }
    }
}
