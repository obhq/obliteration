use crate::errno::Errno;
use crate::fs::{
    make_dev, CdevSw, CharacterDevice, DriverFlags, MakeDev, MakeDevError, Mode, OpenFlags,
};
use crate::process::VThread;
use crate::ucred::{Gid, Uid};
use std::sync::Arc;
use thiserror::Error;

/// Manage all TTY devices.
#[allow(dead_code)]
#[derive(Debug)]
pub struct TtyManager {
    console: Arc<CharacterDevice>, // dev_console
}

impl TtyManager {
    pub fn new() -> Result<Arc<Self>, TtyInitError> {
        // Create /dev/console.
        let console = Arc::new(CdevSw::new(
            DriverFlags::from_bits_retain(0x80000004),
            Self::console_open,
        ));

        let console = make_dev(
            &console,
            0,
            "console",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o600).unwrap(),
            None,
            MakeDev::MAKEDEV_ETERNAL,
        )
        .map_err(TtyInitError::CreateConsoleFailed)?;

        Ok(Arc::new(Self { console }))
    }

    /// See `ttyconsdev_open` on the PS4 for a reference.
    fn console_open(
        _: &Arc<CharacterDevice>,
        _: OpenFlags,
        _: i32,
        _: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        Ok(())
    }
}

/// Represents an error when [`TtyManager`] fails to initialize.
#[derive(Debug, Error)]
pub enum TtyInitError {
    #[error("cannot create console device")]
    CreateConsoleFailed(#[source] MakeDevError),
}
