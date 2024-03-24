use crate::fs::{
    make_dev, CharacterDevice, DeviceDriver, DriverFlags, IoCmd, MakeDevError, MakeDevFlags, Mode,
    OpenFlags, Uio, UioMut,
};
use crate::ucred::{Gid, Uid};
use crate::{errno::Errno, process::VThread};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
pub struct TtyConsole {}

impl TtyConsole {
    pub fn new() -> Self {
        Self {}
    }
}

impl DeviceDriver for TtyConsole {
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

    #[allow(unused_variables)] // TODO: remove when implementing
    fn ioctl(
        &self,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}

/// Manage all TTY devices.
#[allow(dead_code)]
#[derive(Debug)]
pub struct TtyManager {
    console: Arc<CharacterDevice>, // dev_console
}

impl TtyManager {
    pub fn new() -> Result<Arc<Self>, TtyInitError> {
        // Create /dev/console.

        let console = make_dev(
            TtyConsole::new(),
            DriverFlags::from_bits_retain(0x80000004),
            0,
            "console",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o600).unwrap(),
            None,
            MakeDevFlags::MAKEDEV_ETERNAL,
        )?;

        Ok(Arc::new(Self { console }))
    }
}

/// Represents an error when [`TtyManager`] fails to initialize.
#[derive(Debug, Error)]
pub enum TtyInitError {
    #[error("cannot create console device")]
    CreateConsoleFailed(#[from] MakeDevError),
}
