use thiserror::Error;

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

#[derive(Debug)]
struct HmdCmd {}

impl DeviceDriver for HmdCmd {
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
    fn ioctl(
        &self,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}

#[derive(Debug)]
struct HmdSnsr {}

impl DeviceDriver for HmdSnsr {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn ioctl(
        &self,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}

#[derive(Debug)]
struct Hmd3da {}

impl DeviceDriver for Hmd3da {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn ioctl(
        &self,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}

#[derive(Debug)]
struct HmdDist {}

impl DeviceDriver for HmdDist {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn ioctl(
        &self,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}

#[derive(Debug)]
struct HmdCr {}

impl DeviceDriver for HmdCr {
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
    fn ioctl(
        &self,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}

pub struct HmdManager {
    hmd_cmd: Arc<CharacterDevice>,
    hmd_snsr: Arc<CharacterDevice>,
    hmd_3da: Arc<CharacterDevice>,
    hmd_dist: Arc<CharacterDevice>,
    hmd_cr: Arc<CharacterDevice>,
}

impl HmdManager {
    pub fn new() -> Result<Arc<Self>, HmdInitError> {
        let hmd_cmd = make_dev(
            HmdCmd {},
            DriverFlags::INIT,
            0,
            "hmd_cmd",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o666).unwrap(),
            None,
            MakeDevFlags::empty(),
        )
        .map_err(HmdInitError::CreateHmdCmdFailed)?;

        let hmd_snsr = make_dev(
            HmdSnsr {},
            DriverFlags::from_bits_retain(0x80080000),
            0,
            "hmd_snsr",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o666).unwrap(),
            None,
            MakeDevFlags::empty(),
        )
        .map_err(HmdInitError::CreateHmdSnsrFailed)?;

        let hmd_3da = make_dev(
            Hmd3da {},
            DriverFlags::INIT,
            0,
            "hmd_3da",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o666).unwrap(),
            None,
            MakeDevFlags::empty(),
        )
        .map_err(HmdInitError::CreateHmd3daFailed)?;

        let hmd_dist = make_dev(
            HmdDist {},
            DriverFlags::INIT,
            0,
            "hmd_dist",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o666).unwrap(),
            None,
            MakeDevFlags::empty(),
        )
        .map_err(HmdInitError::CreateHmdDistFailed)?;

        let hmd_cr = make_dev(
            HmdCr {},
            DriverFlags::INIT,
            0,
            "hmd_cr",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o600).unwrap(),
            None,
            MakeDevFlags::empty(),
        )
        .map_err(HmdInitError::CreateHmdCrFailed)?;

        Ok(Arc::new(Self {
            hmd_cmd,
            hmd_snsr,
            hmd_3da,
            hmd_dist,
            hmd_cr,
        }))
    }
}

/// Represents an error when [`HmdManager`] fails to initialize.
#[derive(Debug, Error)]
pub enum HmdInitError {
    #[error("cannot create hmd_cmd device")]
    CreateHmdCmdFailed(#[source] MakeDevError),
    #[error("cannot create hmd_snsr device")]
    CreateHmdSnsrFailed(#[source] MakeDevError),
    #[error("cannot create hmd_3da device")]
    CreateHmd3daFailed(#[source] MakeDevError),
    #[error("cannot create hmd_dist device")]
    CreateHmdDistFailed(#[source] MakeDevError),
    #[error("cannot create hmd_cr device")]
    CreateHmdCrFailed(#[source] MakeDevError),
}
