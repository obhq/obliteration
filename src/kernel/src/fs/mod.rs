pub use self::dev::{make_dev, Cdev, CdevSw, DriverFlags, MakeDev, MakeDevError};
pub use self::dirent::*;
pub use self::file::*;
pub use self::ioctl::*;
pub use self::mount::*;
pub use self::path::*;
pub use self::perm::*;
pub use self::vnode::*;
use crate::errno::{Errno, EBADF, EBUSY, EINVAL, ENAMETOOLONG, ENODEV, ENOENT};
use crate::info;
use crate::process::VThread;
use crate::syscalls::{SysArg, SysErr, SysIn, SysOut, Syscalls};
use crate::ucred::PrivilegeError;
use crate::ucred::{Privilege, Ucred};
use bitflags::bitflags;
use gmtx::{Gutex, GutexGroup};
use macros::vpath;
use param::Param;
use std::fmt::{Display, Formatter};
use std::num::{NonZeroI32, TryFromIntError};
use std::path::PathBuf;
use std::sync::{Arc, Weak};
use thiserror::Error;

mod dev;
mod dirent;
mod file;
mod host;
mod ioctl;
mod mount;
mod null;
mod path;
mod perm;
mod tmp;
mod vnode;

/// A virtual filesystem for emulating a PS4 filesystem.
#[derive(Debug)]
pub struct Fs {
    mounts: Gutex<Mounts>,   // mountlist
    root: Gutex<Arc<Vnode>>, // rootvnode
    kern_cred: Arc<Ucred>,
}

impl Fs {
    pub fn new(
        system: impl Into<PathBuf>,
        game: impl Into<PathBuf>,
        param: &Arc<Param>,
        kern_cred: &Arc<Ucred>,
        sys: &mut Syscalls,
    ) -> Result<Arc<Self>, FsError> {
        // Mount devfs as an initial root.
        let mut mounts = Mounts::new();
        let conf = Self::find_config("devfs").unwrap();
        let init = (conf.mount)(
            conf,
            kern_cred,
            vpath!("/dev").to_owned(),
            None,
            MountOpts::new(),
            MountFlags::empty(),
        )
        .map_err(FsError::MountDevFailed)?;

        // Get an initial root vnode.
        let init = mounts.push(init);
        let root = init.root();

        // Setup mount options for root FS.
        let mut opts = MountOpts::new();

        opts.insert("fstype", "exfatfs");
        opts.insert("fspath", VPathBuf::new());
        opts.insert("from", "md0");
        opts.insert("ro", true);
        opts.insert("ob:system", system.into());
        opts.insert("ob:game", game.into());
        opts.insert("ob:param", param.clone());

        // Mount root FS.
        let gg = GutexGroup::new();
        let fs = Arc::new(Self {
            mounts: gg.spawn(mounts),
            root: gg.spawn(root),
            kern_cred: kern_cred.clone(),
        });

        let root = fs
            .mount(opts, MountFlags::MNT_ROOTFS, None)
            .map_err(FsError::MountRootFailed)?;

        // Swap devfs with rootfs so rootfs become an actual root.
        let old = {
            let mut mounts = fs.mounts.write();
            let old = mounts.root().clone();

            mounts.swap(0, 1);
            *fs.root.write() = root.clone();

            old
        };

        // Disconnect rootfs from the root of devfs.
        *old.root().item_mut() = None;
        *fs.mounts.read().root().parent_mut() = None;

        // Set devfs parent to /dev on the root FS.
        let dev = fs
            .lookup(vpath!("/dev"), None)
            .map_err(FsError::LookupDevFailed)?;

        assert!(dev.is_directory());

        {
            let mut p = old.parent_mut();
            assert!(p.is_none());
            *p = Some(dev.clone());
        }

        {
            let mut i = dev.item_mut();
            assert!(i.is_none());
            *i = Some(Arc::new(Arc::downgrade(&old)));
        }

        // Install syscall handlers.
        sys.register(4, &fs, Self::sys_write);
        sys.register(5, &fs, Self::sys_open);
        sys.register(6, &fs, Self::sys_close);
        sys.register(54, &fs, Self::sys_ioctl);
        sys.register(56, &fs, Self::sys_revoke);
        sys.register(136, &fs, Self::sys_mkdir);
        sys.register(209, &fs, Self::sys_poll);
        sys.register(496, &fs, Self::sys_mkdirat);

        Ok(fs)
    }

    pub fn root(&self) -> Arc<Vnode> {
        self.root.read().clone()
    }

    pub fn open(&self, path: impl AsRef<VPath>, td: Option<&VThread>) -> Result<VFile, OpenError> {
        let _vnode = self.lookup(path, td).map_err(OpenError::LookupFailed)?;

        todo!();
    }

    /// This method will **not** follow the last component if it is a mount point or a link.
    pub fn lookup(
        &self,
        path: impl AsRef<VPath>,
        td: Option<&VThread>,
    ) -> Result<Arc<Vnode>, LookupError> {
        // Why we don't follow how namei was implemented? The reason is because:
        //
        // 1. namei is too complicated.
        // 2. namei rely on mutating the nameidata structure, which contribute to its complication.
        //
        // So we decided to implement our own lookup algorithm.
        let path = path.as_ref();
        let mut root = match td {
            Some(td) => td.proc().files().root(),
            None => self.root(),
        };

        // Get starting point.
        let mut vn = if path.is_absolute() {
            root.clone()
        } else if let Some(td) = td {
            td.proc().files().cwd()
        } else {
            root.clone()
        };

        // TODO: Handle link.
        let mut item = root.item_mut();

        match item
            .as_ref()
            .map(|i| i.downcast_ref::<Weak<Mount>>().unwrap())
        {
            Some(m) => match m.upgrade() {
                Some(m) => {
                    drop(item);
                    root = m.root();
                }
                None => {
                    *item = None;
                    drop(item);
                }
            },
            None => drop(item),
        }

        // Walk on path component.
        for (i, com) in path.components().enumerate() {
            // TODO: Handle link.
            match vn.ty() {
                VnodeType::Directory(_) => {
                    let mut item = vn.item_mut();

                    match item
                        .as_ref()
                        .map(|i| i.downcast_ref::<Weak<Mount>>().unwrap())
                    {
                        Some(m) => match m.upgrade() {
                            Some(m) => {
                                drop(item);
                                vn = m.root();
                            }
                            None => {
                                *item = None;
                                drop(item);
                            }
                        },
                        None => drop(item),
                    }
                }
                VnodeType::Character => return Err(LookupError::NotFound),
                _ => todo!(),
            }

            // Prevent ".." on root.
            if com == ".." && Arc::ptr_eq(&vn, &root) {
                return Err(LookupError::NotFound);
            }

            // Lookup next component.
            vn = match vn.lookup(td, com) {
                Ok(v) => v,
                Err(e) => {
                    if e.errno() == ENOENT {
                        return Err(LookupError::NotFound);
                    } else {
                        return Err(LookupError::LookupFailed(
                            i,
                            com.to_owned().into_boxed_str(),
                            e,
                        ));
                    }
                }
            };
        }

        Ok(vn)
    }

    fn sys_write(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let fd: i32 = i.args[0].try_into().unwrap();
        let ptr: *const u8 = i.args[1].into();
        let len: usize = i.args[2].into();

        if len > 0x7fffffff {
            return Err(SysErr::Raw(EINVAL));
        }

        let td = VThread::current().unwrap();
        let file = td.proc().files().get(fd).ok_or(SysErr::Raw(EBADF))?;
        let buf = unsafe { std::slice::from_raw_parts(ptr, len) };
        let written = file.write(buf, Some(&td))?;

        Ok(written.into())
    }

    fn sys_open(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        // Get arguments.
        let path = unsafe { i.args[0].to_path()?.unwrap() };
        let flags: OpenFlags = i.args[1].try_into().unwrap();
        let mode: u32 = i.args[2].try_into().unwrap();

        // Check flags.
        if flags.intersects(OpenFlags::O_EXEC) {
            if flags.intersects(OpenFlags::O_ACCMODE) {
                return Err(SysErr::Raw(EINVAL));
            }
        } else if flags.contains(OpenFlags::O_ACCMODE) {
            return Err(SysErr::Raw(EINVAL));
        }

        // Get full path.
        if flags.intersects(OpenFlags::UNK1) {
            todo!("open({path}) with flags & 0x400000 != 0");
        } else if flags.intersects(OpenFlags::O_SHLOCK) {
            todo!("open({path}) with flags & O_SHLOCK");
        } else if flags.intersects(OpenFlags::O_EXLOCK) {
            todo!("open({path}) with flags & O_EXLOCK");
        } else if flags.intersects(OpenFlags::O_TRUNC) {
            todo!("open({path}) with flags & O_TRUNC");
        } else if mode != 0 {
            todo!("open({path}, {flags}) with mode = {mode}");
        }

        info!("Opening {path} with flags = {flags}.");

        // Lookup file.
        let td = VThread::current().unwrap();
        let mut file = self.open(path, Some(&td))?;

        *file.flags_mut() = flags.into_fflags();

        // Install to descriptor table.
        let fd = td.proc().files().alloc(Arc::new(file));

        info!("File descriptor {fd} was allocated for {path}.");

        Ok(fd.into())
    }

    fn sys_close(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let td = VThread::current().unwrap();
        let fd: i32 = i.args[0].try_into().unwrap();

        info!("Closing fd {fd}.");

        td.proc().files().free(fd)?;

        Ok(SysOut::ZERO)
    }

    fn sys_ioctl(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        const FIOCLEX: IoCmd = IoCmd::io(b'f', 1);
        const FIONCLEX: IoCmd = IoCmd::io(b'f', 2);
        const FIONBIO: IoCmd = IoCmd::iowint(b'f', 0x7e);
        const FIOASYNC: IoCmd = IoCmd::iowint(b'f', 0x7d);

        let fd: i32 = i.args[0].try_into().unwrap();
        let com: IoCmd = i.args[1].try_into()?;
        let data_arg: *mut u8 = i.args[2].into();

        let size: usize = com.size();
        let mut vec = vec![0u8; size];

        // Get data.
        let data = if size == 0 {
            &mut []
        } else {
            if com.is_void() {
                todo!("ioctl with com & IOC_VOID != 0");
            } else {
                &mut vec[..]
            }
        };

        if com.is_in() {
            todo!("ioctl with IOC_IN & != 0");
        } else if com.is_out() {
            data.fill(0);
        }

        // Get target file.
        let td = VThread::current().unwrap();
        let file = td.proc().files().get(fd).ok_or(SysErr::Raw(EBADF))?;

        if !file
            .flags()
            .intersects(VFileFlags::FREAD | VFileFlags::FWRITE)
        {
            return Err(SysErr::Raw(EBADF));
        }

        // Execute the operation.
        info!("Executing ioctl({com}) on file descriptor {fd}.");

        match com {
            FIOCLEX => todo!("ioctl with com = FIOCLEX"),
            FIONCLEX => todo!("ioctl with com = FIONCLEX"),
            FIONBIO => todo!("ioctl with com = FIONBIO"),
            FIOASYNC => todo!("ioctl with com = FIOASYNC"),
            _ => {}
        }

        file.ioctl(com, data, Some(&td))?;

        if com.is_void() {
            unsafe {
                std::ptr::copy_nonoverlapping(data.as_ptr(), data_arg, size);
            }
        }

        Ok(SysOut::ZERO)
    }

    fn sys_revoke(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let path = unsafe { i.args[0].to_path()?.unwrap() };

        info!("Revoking access to {path}.");

        // Check current thread privilege.
        let td = VThread::current().unwrap();

        td.priv_check(Privilege::SCE683)?;

        // TODO: Check vnode::v_rdev.
        let vn = self.lookup(path, Some(&td))?;

        if !vn.is_character() {
            return Err(SysErr::Raw(EINVAL));
        }

        // TODO: It seems like the initial ucred of the process is either root or has PRIV_VFS_ADMIN
        // privilege.
        self.revoke(vn, &td)?;

        Ok(SysOut::ZERO)
    }

    fn revoke(&self, vn: Arc<Vnode>, td: &VThread) -> Result<(), RevokeError> {
        let vattr = vn.getattr().map_err(RevokeError::GetAttrError)?;

        if td.cred().effective_uid() != vattr.uid() {
            td.priv_check(Privilege::VFS_ADMIN)?;
        }

        vn.revoke(RevokeFlags::REVOKE_ALL)
            .map_err(RevokeError::RevokeFailed)?;

        Ok(())
    }

    fn sys_mkdir(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let path = unsafe { i.args[0].to_path() }?.unwrap();
        let mode: u32 = i.args[1].try_into().unwrap();

        let td = VThread::current().unwrap();

        self.mkdirat(At::Cwd, path, mode, Some(&td))
    }

    fn sys_poll(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let fds: *mut PollFd = i.args[0].into();
        let nfds: u32 = i.args[1].try_into().unwrap();
        let timeout: i32 = i.args[2].try_into().unwrap();

        todo!()
    }

    fn sys_mkdirat(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let td = VThread::current().unwrap();

        td.priv_check(Privilege::SCE683)?;

        let fd: i32 = i.args[0].try_into().unwrap();
        let path = unsafe { i.args[1].to_path() }?.unwrap();
        let mode: u32 = i.args[2].try_into().unwrap();

        self.mkdirat(At::Fd(fd), path, mode, Some(&td))
    }

    /// See `kern_mkdirat` on the PS4 for a reference.
    fn mkdirat(
        &self,
        at: At,
        path: &VPath,
        mode: u32, //TODO: probably create a wrapper type for this
        td: Option<&VThread>,
    ) -> Result<SysOut, SysErr> {
        // This will require relative lookups
        todo!()
    }

    /// See `vfs_donmount` on the PS4 for a reference.
    fn mount(
        self: &Arc<Self>,
        mut opts: MountOpts,
        mut flags: MountFlags,
        td: Option<&VThread>,
    ) -> Result<Arc<Vnode>, MountError> {
        // Process the options.
        let fs: Box<str> = opts.remove("fstype").unwrap().unwrap();
        let path: VPathBuf = opts.remove("fspath").unwrap().unwrap();

        opts.retain(|k, v| {
            match k {
                "async" => todo!(),
                "atime" => todo!(),
                "clusterr" => todo!(),
                "clusterw" => todo!(),
                "exec" => todo!(),
                "force" => todo!(),
                "multilabel" => todo!(),
                "noasync" => todo!(),
                "noatime" => todo!(),
                "noclusterr" => todo!(),
                "noclusterw" => todo!(),
                "noexec" => todo!(),
                "noro" => todo!(),
                "nosuid" => todo!(),
                "nosymfollow" => todo!(),
                "rdonly" => todo!(),
                "reload" => todo!(),
                "ro" => flags.set(MountFlags::MNT_RDONLY, v.as_bool().unwrap()),
                "rw" => todo!(),
                "suid" => todo!(),
                "suiddir" => todo!(),
                "symfollow" => todo!(),
                "sync" => todo!(),
                "union" => todo!(),
                "update" => todo!(),
                _ => return true,
            }

            return false;
        });

        if fs.len() >= 15 {
            return Err(MountError::FsTooLong);
        } else if path.len() >= 87 {
            return Err(MountError::PathTooLong);
        }

        // TODO: Apply the remaining checks from the PS4.
        if flags.intersects(MountFlags::MNT_UPDATE) {
            todo!("vfs_donmount with MNT_UPDATE");
        } else {
            let conf = if flags.intersects(MountFlags::MNT_ROOTFS) {
                Self::find_config(fs).ok_or(MountError::InvalidFs)?
            } else {
                todo!("vfs_donmount with !MNT_ROOTFS");
            };

            // Lookup parent vnode.
            let vn = match self.lookup(path.as_ref(), td) {
                Ok(v) => v,
                Err(e) => return Err(MountError::LookupPathFailed(e)),
            };

            // TODO: Check if jailed.

            flags.remove(MountFlags::from_bits_retain(0xFFFFFFFF272F3F80));

            // TODO: Implement budgetid.
            let mount = (conf.mount)(
                conf,
                td.map_or(&self.kern_cred, |t| t.cred()),
                path,
                Some(vn.clone()),
                opts,
                flags,
            )
            .map_err(MountError::MountFailed)?;

            // Set vnode to mounted. Beware of deadlock here.
            let mount = self.mounts.write().push(mount);
            let mut item = vn.item_mut();

            if item.is_some() {
                drop(item);
                self.mounts.write().remove(&mount);
                return Err(MountError::PathAlreadyMounted);
            }

            *item = Some(Arc::new(Arc::downgrade(&mount)));
            drop(item);

            // TODO: Implement the remaining logics from the PS4.
            Ok(mount.root())
        }
    }

    /// See `vfs_byname` on the PS4 for a reference.
    fn find_config(name: impl AsRef<str>) -> Option<&'static FsConfig> {
        let mut name = name.as_ref();
        let mut conf = Some(&HOST);

        if name == "ffs" {
            name = "ufs";
        }

        while let Some(v) = conf {
            if v.name == name {
                return Some(v);
            }

            conf = v.next;
        }

        None
    }
}

bitflags! {
    /// Flags for [`Fs::sys_open()`].
    pub struct OpenFlags: u32 {
        const O_WRONLY = 0x00000001;
        const O_RDWR = 0x00000002;
        const O_ACCMODE = Self::O_WRONLY.bits() | Self::O_RDWR.bits();
        const O_SHLOCK = 0x00000010;
        const O_EXLOCK = 0x00000020;
        const O_TRUNC = 0x00000400;
        const O_EXEC = 0x00040000;
        const O_CLOEXEC = 0x00100000;
        const UNK1 = 0x00400000;
    }
}

impl OpenFlags {
    /// An implementation of `FFLAGS` macro.
    fn into_fflags(self) -> VFileFlags {
        VFileFlags::from_bits_truncate(self.bits() + 1)
    }
}

impl TryFrom<SysArg> for OpenFlags {
    type Error = TryFromIntError;

    fn try_from(value: SysArg) -> Result<Self, Self::Error> {
        Ok(Self::from_bits_retain(value.get().try_into()?))
    }
}

impl Display for OpenFlags {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            f.write_str("empty")
        } else {
            self.0.fmt(f)
        }
    }
}

/// An implementation of `vfsconf` structure.
#[derive(Debug)]
pub struct FsConfig {
    name: &'static str,              // vfc_name
    ty: u32,                         // vfc_typenum
    next: Option<&'static FsConfig>, // vfc_list.next
    mount: fn(
        conf: &'static Self,
        cred: &Arc<Ucred>,
        path: VPathBuf,
        parent: Option<Arc<Vnode>>,
        opts: MountOpts,
        flags: MountFlags,
    ) -> Result<Mount, Box<dyn Errno>>,
}

bitflags! {
    pub struct RevokeFlags: i32 {
        const REVOKE_ALL = 0x0001;
    }
}

#[derive(Debug)]
pub enum At {
    Cwd,
    Fd(i32),
}

struct PollFd {
    fd: i32,
    events: i16,  // TODO: this probably deserves its own type
    revents: i16, // likewise
}

/// Represents an error when FS was failed to initialized.
#[derive(Debug, Error)]
pub enum FsError {
    #[error("cannot mount devfs")]
    MountDevFailed(#[source] Box<dyn Errno>),

    #[error("cannot mount rootfs")]
    MountRootFailed(#[source] MountError),

    #[error("cannot lookup /dev")]
    LookupDevFailed(#[source] LookupError),
}

/// Represents an error when FS mounting is failed.
#[derive(Debug, Error)]
pub enum MountError {
    #[error("fstype is too long")]
    FsTooLong,

    #[error("fspath is too long")]
    PathTooLong,

    #[error("fstype is not valid")]
    InvalidFs,

    #[error("fspath is not found")]
    LookupPathFailed(#[source] LookupError),

    #[error("cannot mount the filesystem")]
    MountFailed(#[source] Box<dyn Errno>),

    #[error("fspath is already mounted")]
    PathAlreadyMounted,
}

impl Errno for MountError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::FsTooLong | Self::PathTooLong => ENAMETOOLONG,
            Self::InvalidFs => ENODEV,
            Self::LookupPathFailed(e) => e.errno(),
            Self::MountFailed(e) => e.errno(),
            Self::PathAlreadyMounted => EBUSY,
        }
    }
}

/// Represents an error when [`Fs::open()`] was failed.
#[derive(Debug, Error)]
pub enum OpenError {
    #[error("cannot lookup the file")]
    LookupFailed(#[source] LookupError),
}

impl Errno for OpenError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::LookupFailed(e) => e.errno(),
        }
    }
}

/// Represents an error when [`Fs::lookup()`] was failed.
#[derive(Debug, Error)]
pub enum LookupError {
    #[error("no such file or directory")]
    NotFound,

    #[error("cannot lookup '{1}' from component #{0}")]
    LookupFailed(usize, Box<str>, #[source] Box<dyn Errno>),
}

impl Errno for LookupError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::NotFound => ENOENT,
            Self::LookupFailed(_, _, e) => e.errno(),
        }
    }
}

#[derive(Debug, Error)]
pub enum RevokeError {
    #[error("failed to get file attr")]
    GetAttrError(#[source] Box<dyn Errno>),

    #[error("insufficient privilege")]
    PrivelegeError(#[from] PrivilegeError),

    #[error("failed to revoke access")]
    RevokeFailed(#[source] Box<dyn Errno>),
}

impl Errno for RevokeError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::GetAttrError(e) => e.errno(),
            Self::PrivelegeError(e) => e.errno(),
            Self::RevokeFailed(e) => e.errno(),
        }
    }
}

static HOST: FsConfig = FsConfig {
    name: "exfatfs",
    ty: 0x2C,
    next: Some(&MLFS),
    mount: self::host::mount,
};

static MLFS: FsConfig = FsConfig {
    name: "mlfs",
    ty: 0xF1,
    next: Some(&UDF2),
    mount: |_, _, _, _, _, _| todo!("mount for mlfs"),
};

static UDF2: FsConfig = FsConfig {
    name: "udf2",
    ty: 0,
    next: Some(&DEVFS),
    mount: |_, _, _, _, _, _| todo!("mount for udf2"),
};

static DEVFS: FsConfig = FsConfig {
    name: "devfs",
    ty: 0x71,
    next: Some(&TMPFS),
    mount: self::dev::mount,
};

static TMPFS: FsConfig = FsConfig {
    name: "tmpfs",
    ty: 0x87,
    next: Some(&UNIONFS),
    mount: self::tmp::mount,
};

static UNIONFS: FsConfig = FsConfig {
    name: "unionfs",
    ty: 0x41,
    next: Some(&PROCFS),
    mount: |_, _, _, _, _, _| todo!("mount for unionfs"),
};

static PROCFS: FsConfig = FsConfig {
    name: "procfs",
    ty: 0x2,
    next: Some(&CD9660),
    mount: |_, _, _, _, _, _| todo!("mount for procfs"),
};

static CD9660: FsConfig = FsConfig {
    name: "cd9660",
    ty: 0xBD,
    next: Some(&UFS),
    mount: |_, _, _, _, _, _| todo!("mount for cd9660"),
};

static UFS: FsConfig = FsConfig {
    name: "ufs",
    ty: 0x35,
    next: Some(&NULLFS),
    mount: |_, _, _, _, _, _| todo!("mount for ufs"),
};

static NULLFS: FsConfig = FsConfig {
    name: "nullfs",
    ty: 0x29,
    next: Some(&PFS),
    mount: self::null::mount,
};

static PFS: FsConfig = FsConfig {
    name: "pfs",
    ty: 0xA4,
    next: None,
    mount: |_, _, _, _, _, _| todo!("mount for pfs"),
};
