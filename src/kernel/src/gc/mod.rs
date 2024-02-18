use std::sync::Arc;

use crate::{
    errno::Errno,
    fs::{make_dev, Cdev, CdevSw, DriverFlags, MakeDev, Mode, OpenFlags},
    process::VThread,
    ucred::{Gid, Uid},
};

pub struct GcManager {}

impl GcManager {
    pub fn new() -> () {
        let gc_devsw = Arc::new(CdevSw::new(DriverFlags::D_INIT, Some(Self::gc_open), None));

        let _gc = make_dev(
            &gc_devsw,
            0,
            "gc",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o666).unwrap(),
            None,
            MakeDev::MAKEDEV_ETERNAL,
        );
    }

    fn gc_open(
        _gc: &Arc<Cdev>,
        _flags: OpenFlags,
        _mode: i32,
        _td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}
