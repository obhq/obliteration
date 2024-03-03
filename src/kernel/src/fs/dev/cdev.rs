use super::dirent::Dirent;
use crate::errno::Errno;
use crate::errno::ENODEV;
use crate::fs::Uio;
use crate::fs::{FileBackend, IoCmd, Mode, OpenFlags, Stat, TruncateLength, UioMut, VFile};
use crate::process::PollEvents;
use crate::process::VThread;
use crate::time::TimeSpec;
use crate::ucred::{Gid, Ucred, Uid};
use bitflags::bitflags;
use gmtx::{Gutex, GutexGroup, GutexReadGuard, GutexWriteGuard};
use macros::Errno;
use std::fmt::Debug;
use std::sync::{Arc, Weak};
use thiserror::Error;

/// An implementation of `cdev` and `cdev_priv` structures.
#[derive(Debug)]
pub struct CharacterDevice {
    sw: Arc<CdevSw>,                           // si_devsw
    unit: i32,                                 // si_drv0
    name: String,                              // si_name
    uid: Uid,                                  // si_uid
    gid: Gid,                                  // si_gid
    mode: Mode,                                // si_mode
    ctime: TimeSpec,                           // si_ctime
    atime: TimeSpec,                           // si_atime
    mtime: TimeSpec,                           // si_mtime
    cred: Option<Arc<Ucred>>,                  // si_cred
    max_io: usize,                             // si_iosize_max
    flags: DeviceFlags,                        // si_flags
    inode: i32,                                // cdp_inode
    dirents: Gutex<Vec<Option<Weak<Dirent>>>>, // cdp_dirents + cdp_maxdirent
}

impl CharacterDevice {
    /// See `devfs_alloc` on the PS4 for a reference.
    pub(super) fn new(
        sw: &Arc<CdevSw>,
        unit: i32,
        name: impl Into<String>,
        uid: Uid,
        gid: Gid,
        mode: Mode,
        cred: Option<Arc<Ucred>>,
        flags: DeviceFlags,
        inode: i32,
    ) -> Self {
        let gg = GutexGroup::new();
        let now = TimeSpec::now();

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

    pub fn uid(&self) -> Uid {
        self.uid
    }

    pub fn gid(&self) -> Gid {
        self.gid
    }

    pub fn mode(&self) -> Mode {
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

impl FileBackend for CharacterDevice {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn read(
        self: &Arc<Self>,
        file: &VFile,
        buf: &mut UioMut,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn write(
        self: &Arc<Self>,
        file: &VFile,
        buf: &mut Uio,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn ioctl(
        self: &Arc<Self>,
        file: &VFile,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn poll(self: &Arc<Self>, file: &VFile, events: PollEvents, td: &VThread) -> PollEvents {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn stat(self: &Arc<Self>, file: &VFile, td: Option<&VThread>) -> Result<Stat, Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn truncate(
        self: &Arc<Self>,
        file: &VFile,
        length: TruncateLength,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
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

pub type CdevOpen =
    fn(&Arc<CharacterDevice>, OpenFlags, i32, Option<&VThread>) -> Result<(), Box<dyn Errno>>;
pub type CdevFd = fn(
    &Arc<CharacterDevice>,
    OpenFlags,
    Option<&VThread>,
    Option<&mut VFile>,
) -> Result<(), Box<dyn Errno>>;

/// An implementation of the `cdevsw` structure.
pub(super) trait Device: Debug + Sync + Send + 'static {
    #[allow(unused_variables)]
    fn read(
        self: Arc<Self>,
        data: &mut UioMut,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        Err(Box::new(DefaultError::ReadNotSupported))
    }

    #[allow(unused_variables)]
    fn write(
        self: Arc<Self>,
        data: &mut Uio,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        Err(Box::new(DefaultError::WriteNotSupported))
    }

    #[allow(unused_variables)]
    fn ioctl(self: Arc<Self>, cmd: IoCmd, td: Option<&VThread>) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(DefaultError::IoctlNotSupported))
    }
}

#[derive(Debug, Error, Errno)]
pub(super) enum DefaultError {
    #[error("read not supported")]
    #[errno(ENODEV)]
    ReadNotSupported,

    #[error("write not supported")]
    #[errno(ENODEV)]
    WriteNotSupported,

    #[error("ioctl not supported")]
    #[errno(ENODEV)]
    IoctlNotSupported,
}
