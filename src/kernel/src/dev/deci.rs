use crate::fs::{
    make_dev, CharacterDevice, DeviceDriver, DriverFlags, MakeDevError, MakeDevFlags, Mode,
    OpenFlags, Uio, UioMut,
};
use crate::ucred::{Gid, Uid};
use crate::{errno::AsErrno, process::VThread};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
struct Driver {}

impl Driver {
    pub fn new() -> Self {
        Self {}
    }
}

impl DeviceDriver for Driver {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn open(
        &self,
        dev: &Arc<CharacterDevice>,
        mode: OpenFlags,
        devtype: i32,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn AsErrno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn read(
        &self,
        dev: &Arc<CharacterDevice>,
        data: &mut UioMut,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn AsErrno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn write(
        &self,
        dev: &Arc<CharacterDevice>,
        data: &mut Uio,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn AsErrno>> {
        todo!()
    }
}

/// Encapsulates a deci device (e.g. `deci_stdout`).
#[allow(dead_code)]
pub struct Deci {
    name: &'static str,
    dev: Arc<CharacterDevice>,
}

impl Deci {
    pub const NAMES: [&'static str; 12] = [
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

    pub(super) fn new(name: &'static str, dev: Arc<CharacterDevice>) -> Self {
        Self { name, dev }
    }
}

/// An implementation of debugging functionalities of the PS4 (not Obliteration debugging).
///
/// It is unclear what deci is stand for. Probably Debug Console Interface?
#[allow(dead_code)]
pub struct DebugManager {
    deci_devs: Vec<Deci>, // decitty_XX
}

impl DebugManager {
    pub fn new() -> Result<Arc<Self>, DebugManagerInitError> {
        // Create deci devices.
        let mut deci_devs = Vec::with_capacity(Deci::NAMES.len());

        for name in Deci::NAMES {
            match make_dev(
                Driver::new(),
                DriverFlags::from_bits_retain(0x80080000),
                0,
                name,
                Uid::ROOT,
                Gid::ROOT,
                Mode::new(0o666).unwrap(),
                None,
                MakeDevFlags::empty(),
            ) {
                Ok(v) => deci_devs.push(Deci::new(name, v)),
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
