use crate::{
    errno::Errno,
    fs::{Device, IoCmd},
    process::VThread,
};
use std::sync::Arc;

#[derive(Debug)]
struct Gc {}

impl Device for Gc {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn ioctl(self: Arc<Self>, cmd: IoCmd, td: &VThread) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}
