use crate::dev::TtyConsDev;
use crate::fs::{make_dev, CharacterDevice, MakeDevError, MakeDevFlags, Mode};
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

        let console = make_dev(
            TtyConsDev::new(),
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
