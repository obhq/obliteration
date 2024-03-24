use super::dirent::Dirent;
use crate::errno::{Errno, ENODEV, ENOTTY};
use crate::fs::Uio;
use crate::fs::{
    FileBackend, IoCmd, Mode, OpenFlags, PollEvents, Stat, TruncateLength, UioMut, VFile,
};
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
    driver: Box<dyn DeviceDriver>,             // si_devsw
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
        unit: i32,
        name: impl Into<String>,
        uid: Uid,
        gid: Gid,
        mode: Mode,
        cred: Option<Arc<Ucred>>,
        flags: DeviceFlags,
        inode: i32,
        driver: impl DeviceDriver,
    ) -> Self {
        let gg = GutexGroup::new();
        let now = TimeSpec::now();

        Self {
            driver: Box::new(driver),
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

    pub fn open(
        self: &Arc<Self>,
        mode: OpenFlags,
        devtype: i32,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        self.driver.open(self, mode, devtype, td)
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

bitflags! {
    /// Flags for [`CdevSw`].
    #[derive(Debug, Clone, Copy)]
    pub struct DriverFlags: u32 {
        const D_NEEDMINOR = 0x00800000;
    }
}

/// An implementation of the `cdevsw` structure.
pub trait DeviceDriver: Debug + Sync + Send + 'static {
    #[allow(unused_variables)]
    fn open(
        &self,
        dev: &Arc<CharacterDevice>,
        mode: OpenFlags,
        devtype: i32,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn read(
        &self,
        dev: &Arc<CharacterDevice>,
        data: &mut UioMut,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        Err(Box::new(DefaultDeviceError::ReadNotSupported))
    }

    #[allow(unused_variables)]
    fn write(
        &self,
        dev: &Arc<CharacterDevice>,
        data: &mut Uio,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        Err(Box::new(DefaultDeviceError::WriteNotSupported))
    }

    #[allow(unused_variables)]
    fn ioctl(
        &self,
        dev: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(DefaultDeviceError::IoctlNotSupported))
    }
}

#[derive(Debug, Error, Errno)]
pub enum DefaultDeviceError {
    #[error("read not supported")]
    #[errno(ENODEV)]
    ReadNotSupported,

    #[error("write not supported")]
    #[errno(ENODEV)]
    WriteNotSupported,

    #[error("ioctl not supported")]
    #[errno(ENODEV)]
    IoctlNotSupported,

    #[error("command not supported")]
    #[errno(ENOTTY)]
    CommandNotSupported,
}
