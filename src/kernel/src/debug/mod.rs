use self::deci::DeciDev;
use crate::dev::Deci;
use crate::fs::{make_dev, MakeDevError, MakeDevFlags, Mode};
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

        for name in DeciDev::NAMES {
            match make_dev(
                Deci::new(),
                0,
                name,
                Uid::ROOT,
                Gid::ROOT,
                Mode::new(0o666).unwrap(),
                None,
                MakeDevFlags::empty(),
            ) {
                Ok(v) => deci_devs.push(DeciDev::new(name, v)),
                Err(e) => return Err(DebugManagerInitError::CreateDeciFailed(name, e)),
            }
        }

        Ok(Arc::new(Self { deci_devs }))
    }
}

/// Represents an error when [`DebugManager`] is failed to initialize.
#[derive(Debug, Error)]
pub enum DebugManagerInitError {
    #[error("couldn't create {0}")]
    CreateDeciFailed(&'static str, #[source] MakeDevError),
}
