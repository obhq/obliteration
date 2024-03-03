use crate::{
    errno::Errno,
    fs::{dev::Device, IoCmd, Uio, UioMut},
    process::VThread,
};
use std::sync::Arc;

#[derive(Debug)]
struct Random {}

impl Device for Random {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn read(
        self: Arc<Self>,
        data: &mut UioMut,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn write(
        self: Arc<Self>,
        data: &mut Uio,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn ioctl(self: Arc<Self>, cmd: IoCmd, td: Option<&VThread>) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}
