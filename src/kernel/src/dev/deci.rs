use crate::fs::{CharacterDevice, DeviceDriver, OpenFlags, Uio, UioMut};
use crate::{errno::Errno, process::VThread};
use std::sync::Arc;

#[derive(Debug)]
pub struct Deci {}

impl Deci {
    pub fn new() -> Self {
        Self {}
    }
}

impl DeviceDriver for Deci {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn open(
        &self,
        dev: &Arc<CharacterDevice>,
        mode: OpenFlags,
        devtype: i32,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn read(
        &self,
        dev: &Arc<CharacterDevice>,
        data: &mut UioMut,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn write(
        &self,
        dev: &Arc<CharacterDevice>,
        data: &mut Uio,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        todo!()
    }
}
