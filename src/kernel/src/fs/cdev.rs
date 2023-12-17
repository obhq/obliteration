use crate::ucred::Ucred;
use bitflags::bitflags;
use std::sync::Arc;
use std::time::SystemTime;

/// An implementation of `cdev` and `cdev_priv` structures.
#[derive(Debug)]
pub struct Cdev {
    sw: Arc<CdevSw>,          // si_devsw
    inode: u32,               // cdp_inode
    unit: i32,                // si_drv0
    name: String,             // si_name
    uid: i32,                 // si_uid
    gid: i32,                 // si_gid
    mode: u16,                // si_mode
    ctime: SystemTime,        // si_ctime
    atime: SystemTime,        // si_atime
    mtime: SystemTime,        // si_mtime
    cred: Option<Arc<Ucred>>, // si_cred
    flags: DeviceFlags,       // si_flags
}

impl Cdev {
    /// See `devfs_alloc` on the PS4 for a reference.
    pub(super) fn new<N: Into<String>>(
        sw: &Arc<CdevSw>,
        inode: u32,
        unit: i32,
        name: N,
        uid: i32,
        gid: i32,
        mode: u16,
        cred: Option<Arc<Ucred>>,
        flags: DeviceFlags,
    ) -> Self {
        let now = SystemTime::now();

        Self {
            sw: sw.clone(),
            inode,
            unit,
            name: name.into(),
            uid,
            gid,
            mode,
            ctime: now,
            atime: now,
            mtime: now,
            cred,
            flags,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn flags(&self) -> DeviceFlags {
        self.flags
    }
}

bitflags! {
    /// Flags for [`Cdev`].
    #[derive(Debug, Clone, Copy)]
    pub struct DeviceFlags: u32 {
        const SI_ETERNAL = 0x0001;
    }
}

/// An implementation of `cdevsw` structure.
#[derive(Debug)]
pub struct CdevSw {
    flags: DriverFlags, // d_flags
}

impl CdevSw {
    /// See `prep_cdevsw` on the PS4 for a reference.
    pub fn new(flags: DriverFlags) -> Self {
        Self { flags }
    }

    pub fn flags(&self) -> DriverFlags {
        self.flags
    }
}

bitflags! {
    /// Flags for [`CdevSw`].
    #[derive(Debug, Clone, Copy)]
    pub struct DriverFlags: u32 {
        const D_NEEDMINOR = 0x00800000;
    }
}
