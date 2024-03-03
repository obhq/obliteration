use crate::{
    errno::Errno,
    fs::{dev::Device, IoCmd},
    process::VThread,
};
use std::sync::Arc;

#[derive(Debug)]
struct Gc {}

impl Device for Gc {
    fn ioctl(self: Arc<Self>, cmd: IoCmd, td: Option<&VThread>) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}
