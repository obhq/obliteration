use crate::errno::Errno;
use crate::fs::{
    make_dev, Cdev, CdevSw, DriverFlags, Fs, IoCmd, MakeDev, MakeDevError, Mode, OpenFlags,
};
use crate::process::VThread;
use crate::ucred::{Gid, Uid};
use std::sync::Arc;
use thiserror::Error;

/// Manage all TTY devices.
#[derive(Debug)]
pub struct TtyManager {
    console: Arc<Cdev>, // dev_console
}

impl TtyManager {
    const TIOCSCTTY: IoCmd = IoCmd::io(b't', 97);

    pub fn new(_fs: &Arc<Fs>) -> Result<Arc<Self>, TtyError> {
        // Create /dev/console.
        let console = Arc::new(CdevSw::new(
            DriverFlags::from_bits_retain(0x80000004),
            Some(Self::console_open),
            None,
        ));

        let console = match make_dev(
            &console,
            0,
            "console",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o600).unwrap(),
            None,
            MakeDev::MAKEDEV_ETERNAL,
        ) {
            Ok(v) => v,
            Err(e) => return Err(TtyError::CreateConsoleFailed(e)),
        };

        Ok(Arc::new(Self { console }))
    }

    /// See `ttyconsdev_open` on the PS4 for a reference.
    fn console_open(
        _: &Arc<Cdev>,
        _: OpenFlags,
        _: i32,
        _: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}

/// Represents an error when [`TtyManager`] was failed to initialized.
#[derive(Debug, Error)]
pub enum TtyError {
    #[error("cannot create console device")]
    CreateConsoleFailed(#[source] MakeDevError),
}
