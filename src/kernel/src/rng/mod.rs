use std::sync::Arc;

use crate::{
    errno::Errno,
    fs::{make_dev, Cdev, CdevSw, DriverFlags, MakeDev, Mode, OpenFlags},
    process::VThread,
    ucred::{Gid, Uid},
};

pub struct RngManager {}

impl RngManager {
    pub fn new() -> () {
        let rng_cdevsw = Arc::new(CdevSw::new(DriverFlags::D_INIT, Some(Self::rng_open), None));

        let _rng = make_dev(
            &rng_cdevsw,
            0,
            "rng",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o444).unwrap(),
            None,
            MakeDev::MAKEDEV_ETERNAL,
        );
    }

    fn rng_open(
        _rng: &Arc<Cdev>,
        _flags: OpenFlags,
        _mode: i32,
        _td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        // true to PS4
        Ok(())
    }
}
