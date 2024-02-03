use std::sync::Arc;

use crate::{
    errno::Errno,
    fs::{make_dev, Cdev, CdevSw, DriverFlags, MakeDev, Mode, OpenFlags},
    process::VThread,
    ucred::{Gid, Uid},
};

#[derive(Debug)]
pub struct SblManager {}

impl SblManager {
    pub fn new() -> Arc<Self> {
        let sbl = Arc::new(Self {});

        let sbl_srv_cdevsw = Arc::new(CdevSw::new(
            DriverFlags::D_INIT,
            Some(Self::sbl_srv_open),
            None,
        ));

        let _ = make_dev(
            &sbl_srv_cdevsw,
            0,
            "sbl_srv",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o600).unwrap(),
            None,
            MakeDev::MAKEDEV_ETERNAL,
        );

        sbl
    }

    fn sbl_srv_open(
        _: &Arc<Cdev>,
        _: OpenFlags,
        _: i32,
        _: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}
