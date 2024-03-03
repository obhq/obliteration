use crate::{
    errno::Errno,
    fs::{dev::Device, Uio, UioMut},
    process::VThread,
};
use std::sync::Arc;

#[derive(Debug)]
struct Deci {}

impl Device for Deci {
    fn read(
        self: Arc<Self>,
        data: &mut UioMut,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        todo!()
    }

    fn write(
        self: Arc<Self>,
        data: &mut Uio,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        todo!()
    }
}
