use crate::{
    errno::Errno,
    fs::{Device, IoCmd},
    process::VThread,
};
use std::sync::Arc;

#[derive(Debug)]
struct SblSrv {}

impl Device for SblSrv {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn ioctl(self: Arc<Self>, cmd: IoCmd, td: Option<&VThread>) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}
