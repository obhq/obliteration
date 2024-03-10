use crate::fs::{Device, IoCmd, Uio, UioMut};
use crate::{errno::Errno, process::VThread};
use std::sync::Arc;

#[derive(Debug)]
struct TtyConsDev {}

impl Device for TtyConsDev {
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
    fn ioctl(self: Arc<Self>, cmd: IoCmd, td: &VThread) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}
