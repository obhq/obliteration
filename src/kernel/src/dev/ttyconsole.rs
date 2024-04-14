use crate::fs::{
    make_dev, CharacterDevice, DeviceDriver, DriverFlags, IoCmd, MakeDevError, MakeDevFlags, Mode,
    OpenFlags, Uio, UioMut,
};
use crate::ucred::{Gid, Uid};
use crate::{errno::Errno, process::VThread};
use macros::Errno;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
pub struct TtyConsole {
    tty: Tty,
}

impl TtyConsole {
    pub fn new() -> Self {
        Self { tty: Tty::new() }
    }
}

impl DeviceDriver for TtyConsole {
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
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn write(
        &self,
        dev: &Arc<CharacterDevice>,
        data: &mut Uio,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    /// See `ttydev_ioctl` on the PS4 for a reference.
    fn ioctl(
        &self,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        // TODO: implement tty_wait_background

        match cmd {
            IoCmd::TIOCSCTTY => self.tty.ioctl(cmd, td)?,
            _ => todo!(),
        }

        Ok(())
    }
}

#[derive(Debug)]
struct Tty {}

impl Tty {
    fn new() -> Self {
        Self {}
    }

    /// See `tty_ioctl` on the PS4 for a reference.
    fn ioctl(&self, cmd: IoCmd, td: Option<&VThread>) -> Result<(), TtyIoctlError> {
        // TODO: implement ttydevsw_ioctl

        self.generic_ioctl(cmd, td)
    }

    /// See `tty_generic_ioctl` on the PS4 for a reference.
    fn generic_ioctl(&self, cmd: IoCmd, _td: Option<&VThread>) -> Result<(), TtyIoctlError> {
        match cmd {
            IoCmd::TIOCSCTTY => todo!(),
            _ => todo!(),
        }
    }
}

/// Manage all TTY devices.
#[allow(dead_code)]
#[derive(Debug)]
pub struct TtyManager {
    console: Arc<CharacterDevice>, // dev_console
}

impl TtyManager {
    pub fn new() -> Result<Arc<Self>, TtyManagerInitError> {
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
pub enum TtyManagerInitError {
    #[error("cannot create console device")]
    CreateConsoleFailed(#[from] MakeDevError),
}

/// Represents an error when [`Tty::ioctl`] fails to initialize.
#[derive(Debug, Error, Errno)]
pub enum TtyIoctlError {}
