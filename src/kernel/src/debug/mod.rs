use self::deci::DeciDev;
use crate::errno::Errno;
use crate::fs::{
    make_dev, Cdev, CdevSw, DriverFlags, IoCmd, MakeDev, MakeDevError, Mode, OpenFlags,
};
use crate::process::VThread;
use crate::ucred::{Gid, Uid};
use std::sync::Arc;
use thiserror::Error;

mod deci;

/// An implementation of debugging functionalities of the PS4 (not Obliteration debugging).
///
/// It is unclear what deci is stand for. Probably Debug Console Interface?
#[allow(dead_code)]
pub struct DebugManager {
    deci_devs: Vec<DeciDev>, // decitty_XX
}

impl DebugManager {
    pub fn new() -> Result<Arc<Self>, DebugManagerInitError> {
        // Create deci devices.
        let mut deci_devs = Vec::with_capacity(DeciDev::NAMES.len());
        let sw = Arc::new(CdevSw::new(
            DriverFlags::from_bits_retain(0x80080000),
            Some(Self::deci_open),
            None,
            Self::deci_ioctl,
        ));

        for name in DeciDev::NAMES {
            match make_dev(
                &sw,
                0,
                name,
                Uid::ROOT,
                Gid::ROOT,
                Mode::new(0o666).unwrap(),
                None,
                MakeDev::empty(),
            ) {
                Ok(v) => deci_devs.push(DeciDev::new(name, v)),
                Err(e) => return Err(DebugManagerInitError::CreateDeciFailed(name, e)),
            }
        }

        Ok(Arc::new(Self { deci_devs }))
    }

    fn deci_open(
        _: &Arc<Cdev>,
        _: OpenFlags,
        _: i32,
        _: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }

    fn deci_ioctl(
        _: &Arc<Cdev>,
        _: IoCmd,
        _: &mut [u8],
        _: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}

/// Represents an error when [`DebugManager`] is failed to initialize.
#[derive(Debug, Error)]
pub enum DebugManagerInitError {
    #[error("couldn't create {0}")]
    CreateDeciFailed(&'static str, #[source] MakeDevError),
}
