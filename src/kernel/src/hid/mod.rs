use std::sync::Arc;

use crate::{
    errno::Errno,
    fs::{make_dev, Cdev, CdevSw, DriverFlags, MakeDev, Mode, OpenFlags},
    process::VThread,
    ucred::{Gid, Uid},
};

pub struct HidManager {}

impl HidManager {
    pub fn new() -> () {
        let hid_cdevsw = Arc::new(CdevSw::new(
            DriverFlags::D_INIT | DriverFlags::D_TRACKCLOSE,
            Some(Self::hid_open),
            None,
        ));

        let _hid = make_dev(
            &hid_cdevsw,
            0,
            "hid",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o666).unwrap(),
            None,
            MakeDev::MAKEDEV_ETERNAL,
        );
    }

    fn hid_open(
        _hid: &Arc<Cdev>,
        _flags: OpenFlags,
        _mode: i32,
        _td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}
