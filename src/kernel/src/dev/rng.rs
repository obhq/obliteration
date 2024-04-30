use crate::{
    arnd,
    errno::Errno,
    fs::{
        make_dev, CharacterDevice, DeviceDriver, DriverFlags, IoCmd, MakeDevError, MakeDevFlags,
        Mode,
    },
    process::VThread,
    ucred::{Gid, Uid},
};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
struct Rng {}

impl Rng {
    fn new() -> Self {
        Self {}
    }
}

impl DeviceDriver for Rng {
    fn ioctl(
        &self,
        _: &Arc<CharacterDevice>,
        cmd: IoCmd,
        _: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        match cmd {
            // TODO: these are separate algorithms, and should be implemented as such,
            // however arc4rand seems sufficient for now
            IoCmd::RNGGETGENUINE(input) | IoCmd::RNGFIPS(input) => {
                input.error = 0;

                arnd::rand_bytes(&mut input.data);

                Ok(())
            }
            _ => todo!(), // ENOIOCTL,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct RngInput {
    /// This field seems to be treated as an error
    error: i32,
    data: [u8; 64],
}

pub struct RngManager {
    rng: Arc<CharacterDevice>,
}

impl RngManager {
    pub fn new() -> Result<Arc<Self>, RngInitError> {
        let rng = make_dev(
            Rng::new(),
            DriverFlags::from_bits_retain(0x80000004),
            0,
            "rng",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o444).unwrap(),
            None,
            MakeDevFlags::ETERNAL,
        )?;

        Ok(Arc::new(Self { rng }))
    }
}

/// Represents an error when [`RngManager`] fails to initialize.
#[derive(Debug, Error)]
pub enum RngInitError {
    #[error("cannot create rng device")]
    CreateRngFailed(#[from] MakeDevError),
}
