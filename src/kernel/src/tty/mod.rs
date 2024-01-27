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
    console: Arc<Cdev>,       // dev_console
    deci_tty: Vec<Arc<Cdev>>, // decitty_XX
}

impl TtyManager {
    const TIOCSCTTY: IoCmd = IoCmd::io(b't', 97);

    pub fn new(fs: &Arc<Fs>) -> Result<Arc<Self>, TtyInitError> {
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
            Err(e) => return Err(TtyInitError::CreateConsoleFailed(e)),
        };

        let decitty = Arc::new(CdevSw::new(
            DriverFlags::from_bits_retain(0x80080000),
            Some(Self::decitty_open),
            None,
        ));

        let decitty_names: Vec<&str> = vec![
            "deci_stdout",
            "deci_stderr",
            "deci_tty2",
            "deci_tty3",
            "deci_tty4",
            "deci_tty5",
            "deci_tty6",
            "deci_tty7",
            "deci_ttya0",
            "deci_ttyb0",
            "deci_ttyc0",
            "deci_coredump",
        ];

        let deci_tty = decitty_names
            .into_iter()
            .map(|name| {
                make_dev(
                    &decitty,
                    0,
                    name,
                    Uid::ROOT,
                    Gid::ROOT,
                    Mode::new(0o666).unwrap(),
                    None,
                    MakeDev::MAKEDEV_ETERNAL,
                )
                .unwrap()
            })
            .collect();

        Ok(Arc::new(Self { console, deci_tty }))
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

    fn decitty_open(
        _: &Arc<Cdev>,
        _: OpenFlags,
        _: i32,
        _: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}

/// Represents an error when [`TtyManager`] fails to initialize.
#[derive(Debug, Error)]
pub enum TtyInitError {
    #[error("cannot create console device")]
    CreateConsoleFailed(#[source] MakeDevError),
}
