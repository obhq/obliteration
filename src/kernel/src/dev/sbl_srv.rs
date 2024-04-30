use crate::{
    errno::Errno,
    fs::{
        make_dev, CharacterDevice, DeviceDriver, DriverFlags, IoCmd, MakeDevError, MakeDevFlags,
        Mode,
    },
    process::VThread,
    ucred::{Gid, Uid},
};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
struct SblSrv {}

impl SblSrv {
    fn new() -> Self {
        Self {}
    }
}

impl DeviceDriver for SblSrv {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn ioctl(
        &self,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}

pub struct SblSrvManager {
    sbl: Arc<CharacterDevice>,
}

impl SblSrvManager {
    pub fn new() -> Result<Arc<Self>, SblSrvInitError> {
        let sbl = make_dev(
            SblSrv {},
            DriverFlags::from_bits_retain(0x80000004),
            0,
            "sbl_srv",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o444).unwrap(),
            None,
            MakeDevFlags::ETERNAL,
        )?;

        Ok(Arc::new(Self { sbl }))
    }
}

/// Represents an error when [`SblSrvManager`] fails to initialize.
#[derive(Debug, Error)]
pub enum SblSrvInitError {
    #[error("cannot create sbl_srv device")]
    CreateSblSrvFailed(#[from] MakeDevError),
}
