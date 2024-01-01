use super::dirent::Dirent;
use crate::errno::Errno;
use crate::fs::{OpenFlags, VFile};
use crate::process::VThread;
use crate::ucred::Ucred;
use bitflags::bitflags;
use gmtx::{Gutex, GutexGroup, GutexReadGuard, GutexWriteGuard};
use std::sync::{Arc, Weak};
use std::time::SystemTime;

/// An implementation of `cdev` and `cdev_priv` structures.
#[derive(Debug)]
pub struct Cdev {
    sw: Arc<CdevSw>,                           // si_devsw
    unit: i32,                                 // si_drv0
    name: String,                              // si_name
    uid: i32,                                  // si_uid
    gid: i32,                                  // si_gid
    mode: u16,                                 // si_mode
    ctime: SystemTime,                         // si_ctime
    atime: SystemTime,                         // si_atime
    mtime: SystemTime,                         // si_mtime
    cred: Option<Arc<Ucred>>,                  // si_cred
    max_io: usize,                             // si_iosize_max
    flags: DeviceFlags,                        // si_flags
    inode: i32,                                // cdp_inode
    dirents: Gutex<Vec<Option<Weak<Dirent>>>>, // cdp_dirents + cdp_maxdirent
}

impl Cdev {
    /// See `devfs_alloc` on the PS4 for a reference.
    pub(super) fn new<N: Into<String>>(
        sw: &Arc<CdevSw>,
        unit: i32,
        name: N,
        uid: i32,
        gid: i32,
        mode: u16,
        cred: Option<Arc<Ucred>>,
        flags: DeviceFlags,
        inode: i32,
    ) -> Self {
        let gg = GutexGroup::new();
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
            max_io: 0x10000,
            flags,
            dirents: gg.spawn(vec![None]),
        }
    }

    pub fn sw(&self) -> &CdevSw {
        self.sw.as_ref()
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn uid(&self) -> i32 {
        self.uid
    }

    pub fn gid(&self) -> i32 {
        self.gid
    }

    pub fn mode(&self) -> u16 {
        self.mode
    }

    pub fn flags(&self) -> DeviceFlags {
        self.flags
    }

    pub(super) fn inode(&self) -> i32 {
        self.inode
    }

    pub(super) fn dirents(&self) -> GutexReadGuard<Vec<Option<Weak<Dirent>>>> {
        self.dirents.read()
    }

    pub(super) fn dirents_mut(&self) -> GutexWriteGuard<Vec<Option<Weak<Dirent>>>> {
        self.dirents.write()
    }
}

bitflags! {
    /// Flags for [`Cdev`].
    #[derive(Debug, Clone, Copy)]
    pub struct DeviceFlags: u32 {
        const SI_ETERNAL = 0x01;
        const SI_ALIAS = 0x02;
    }
}

/// An implementation of `cdevsw` structure.
#[derive(Debug)]
pub struct CdevSw {
    flags: DriverFlags,     // d_flags
    open: Option<CdevOpen>, // d_open
    fdopen: Option<CdevFd>, // d_fdopen
}

impl CdevSw {
    /// See `prep_cdevsw` on the PS4 for a reference.
    pub fn new(flags: DriverFlags, open: Option<CdevOpen>, fdopen: Option<CdevFd>) -> Self {
        Self {
            flags,
            open,
            fdopen,
        }
    }

    pub fn flags(&self) -> DriverFlags {
        self.flags
    }

    pub fn open(&self) -> Option<CdevOpen> {
        self.open
    }

    pub fn fdopen(&self) -> Option<CdevFd> {
        self.fdopen
    }
}

bitflags! {
    /// Flags for [`CdevSw`].
    #[derive(Debug, Clone, Copy)]
    pub struct DriverFlags: u32 {
        const D_NEEDMINOR = 0x00800000;
    }
}

pub type CdevOpen = fn(&Arc<Cdev>, OpenFlags, i32, Option<&VThread>) -> Result<(), Box<dyn Errno>>;
pub type CdevFd =
    fn(&Arc<Cdev>, OpenFlags, Option<&VThread>, Option<&mut VFile>) -> Result<(), Box<dyn Errno>>;
