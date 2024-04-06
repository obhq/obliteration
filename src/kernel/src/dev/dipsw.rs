use thiserror::Error;

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

#[derive(Debug)]
struct Dipsw {}

impl Dipsw {
    fn new() -> Self {
        Self {}
    }
}

impl DeviceDriver for Dipsw {
    #[allow(unused_variables)]
    fn ioctl(
        &self,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        let td = td.unwrap();

        if !td.cred().is_system() {
            match cmd {
                // TODO: properly implement this
                IoCmd::DIPSWCHECK2(val) => *val = false as i32,
                _ => todo!(),
            }
        } else {
            todo!()
        }

        Ok(())
    }
}

pub struct DipswManager {
    dipsw: Arc<CharacterDevice>,
}

impl DipswManager {
    pub fn new() -> Result<Arc<Self>, DipswInitError> {
        let dipsw = make_dev(
            Dipsw::new(),
            DriverFlags::from_bits_retain(0x80000004),
            0,
            "dipsw",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o644).unwrap(),
            None,
            MakeDevFlags::MAKEDEV_ETERNAL,
        )?;

        Ok(Arc::new(Self { dipsw }))
    }
}

/// Represents an error when [`TtyManager`] fails to initialize.
#[derive(Debug, Error)]
pub enum DipswInitError {
    #[error("cannot create dipsw device")]
    CreateConsoleFailed(#[from] MakeDevError),
}
