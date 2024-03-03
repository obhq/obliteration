use crate::{
    errno::Errno,
    fs::{dev::Device, IoCmd},
    process::VThread,
};
use std::sync::Arc;

#[derive(Debug)]
struct Dipsw {}

impl Device for Dipsw {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn ioctl(self: Arc<Self>, cmd: IoCmd, td: Option<&VThread>) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}
