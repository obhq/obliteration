use crate::{
    errno::Errno,
    fs::{
        make_dev, CharacterDevice, DeviceDriver, DriverFlags, IoCmd, MakeDevError, MakeDevFlags,
        Mode, OpenFlags,
    },
    process::VThread,
    ucred::{Gid, Uid},
};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
struct Gc {}

impl Gc {
    fn new() -> Self {
        Self {}
    }
}

impl DeviceDriver for Gc {
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

    fn ioctl(
        &self,
        _: &Arc<CharacterDevice>,
        cmd: IoCmd,
        _: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        match cmd {
            IoCmd::GCGETCUMASK(_) => todo!("GCGETCUMASK ioctl"),
            IoCmd::GCSETGSRINGSIZES(_) => todo!("GCSETGSRINGSIZES ioctl"),
            IoCmd::GCMIPSTATSREPORT(_) => todo!("GCMIPSTATSREPORT ioctl"),
            IoCmd::GC27(_) => todo!("GC27 ioctl"),
            IoCmd::GCGETNUMTCAUNITS(_) => todo!("GCGETNUMTCAUNITS ioctl"),
            IoCmd::GCDINGDONGFORWORKLOAD(_) => todo!("GCDINGDONGFORWORKLOAD ioctl"),
            IoCmd::GCMAPCOMPUTEQUEUE(_) => todo!("GCMAPCOMPUTEQUEUE ioctl"),
            IoCmd::GCUNMAPCOMPUTEQUEUE(_) => todo!("GCUNMAPCOMPUTEQUEUE ioctl"),
            IoCmd::GCSETWAVELIMITMULTIPLIER(_) => todo!("GCSETWAVELIMITMULTIPLIER ioctl"),
            _ => todo!(),
        }
    }
}

pub struct GcManager {
    gc: Arc<CharacterDevice>,
}

impl GcManager {
    pub fn new() -> Result<Arc<Self>, GcInitError> {
        let gc = make_dev(
            Gc::new(),
            DriverFlags::from_bits_retain(0x80000004),
            0,
            "gc",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o666).unwrap(),
            None,
            MakeDevFlags::MAKEDEV_ETERNAL,
        )?;

        Ok(Arc::new(Self { gc }))
    }
}

/// Represents an error when [`GcManager`] fails to initialize.
#[derive(Debug, Error)]
pub enum GcInitError {
    #[error("cannot create gc device")]
    CreateGcFailed(#[from] MakeDevError),
}
