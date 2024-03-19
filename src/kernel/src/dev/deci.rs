use crate::fs::{CharacterDevice, Device, Uio, UioMut};
use crate::{errno::Errno, process::VThread};
use std::sync::Arc;

#[derive(Debug)]
struct Deci {}

impl Device for Deci {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn read(
        self: &Arc<Self>,
        dev: &Arc<CharacterDevice>,
        data: &mut UioMut,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn write(
        self: &Arc<Self>,
        dev: &Arc<CharacterDevice>,
        data: &mut Uio,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        todo!()
    }
}
