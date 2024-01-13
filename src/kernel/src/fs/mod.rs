pub use self::dev::*;
pub use self::dirent::*;
pub use self::file::*;
pub use self::host::*;
pub use self::ioctl::*;
pub use self::mount::*;
pub use self::path::*;
pub use self::perm::*;
pub use self::vnode::*;
use crate::errno::{Errno, EBADF, EBUSY, EINVAL, ENAMETOOLONG, ENODEV, ENOENT};
use crate::info;
use crate::process::VThread;
use crate::syscalls::{SysArg, SysErr, SysIn, SysOut, Syscalls};
use crate::ucred::{Privilege, Ucred};
use bitflags::bitflags;
use gmtx::{Gutex, GutexGroup};
use macros::vpath;
use param::Param;
use std::any::Any;
use std::collections::HashMap;
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
mod path;
mod perm;
mod tmp;
mod vnode;

/// A virtual filesystem for emulating a PS4 filesystem.
#[derive(Debug)]
pub struct Fs {
    mounts: Gutex<Mounts>,   // mountlist
    root: Gutex<Arc<Vnode>>, // rootvnode
    kern: Arc<Ucred>,
}

impl Fs {
    pub fn new<S, G>(
        system: S,
        game: G,
        param: &Arc<Param>,
        kern: &Arc<Ucred>,
        sys: &mut Syscalls,
    ) -> Result<Arc<Self>, FsError>
    where
        S: Into<PathBuf>,
        G: Into<PathBuf>,
    {
        // Mount devfs as an initial root.
        let mut mounts = Mounts::new();
        let conf = Self::find_config("devfs").unwrap();
        let mut init = Mount::new(None, conf, "/dev", kern);

        if let Err(e) = (init.fs().ops.mount)(&mut init, HashMap::new()) {
            return Err(FsError::MountDevFailed(e));
        }

        // Get an initial root vnode.
        let root = (init.fs().ops.root)(&mounts.push(init));

        // Setup mount options for root FS.
        let mut opts: HashMap<String, Box<dyn Any>> = HashMap::new();

        opts.insert("fstype".into(), Box::new(String::from("exfatfs")));
        opts.insert("fspath".into(), Box::new(VPathBuf::new()));
        opts.insert("from".into(), Box::new(String::from("md0")));
        opts.insert("ro".into(), Box::new(true));
        opts.insert("ob:system".into(), Box::new(system.into()));
        opts.insert("ob:game".into(), Box::new(game.into()));
        opts.insert("ob:param".into(), Box::new(param.clone()));

        // Mount root FS.
        let gg = GutexGroup::new();
        let fs = Arc::new(Self {
            mounts: gg.spawn(mounts),
            root: gg.spawn(root),
            kern: kern.clone(),
        });

        let root = match fs.mount(opts, MountFlags::MNT_ROOTFS, None) {
            Ok(v) => v,
            Err(e) => return Err(FsError::MountRootFailed(e)),
        };

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
            .map_err(|e| FsError::LookupDevFailed(e))?;

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

        Ok(fs)
    }

    pub fn app(&self) -> Arc<VPathBuf> {
        let root = self.mounts.read().root().clone();
        let data = root.data().cloned();
        let host = data.unwrap().downcast::<HostFs>().unwrap();

        host.app().clone()
    }

    pub fn root(&self) -> Arc<Vnode> {
        self.root.read().clone()
    }

    pub fn open<P: AsRef<VPath>>(&self, path: P, td: Option<&VThread>) -> Result<VFile, OpenError> {
        todo!()
    }

    /// This method will **not** follow the last component if it is a mount point or a link.
    pub fn lookup<P: AsRef<VPath>>(
        &self,
        path: P,
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
                        return Err(LookupError::LookupFailed(i, com.to_owned(), e));
                    }
                }
            };
        }

        Ok(vn)
    }

    fn revoke<P: Into<VPathBuf>>(&self, _path: P) {
        // TODO: Implement this.
    }

    fn sys_write(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let fd: i32 = i.args[0].try_into().unwrap();
        let ptr: *const u8 = i.args[1].into();
        let len: usize = i.args[2].try_into().unwrap();

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

        *file.flags_mut() = flags.to_fflags();

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
        const UNK_COM1: IoCmd = IoCmd::io(b'f', 1);
        const UNK_COM2: IoCmd = IoCmd::io(b'f', 2);
        const UNK_COM3: IoCmd = IoCmd::iowint(b'f', 0x7e);
        const UNK_COM4: IoCmd = IoCmd::iowint(b'f', 0x7d);

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
            UNK_COM1 => todo!("ioctl with com = 0x20006601"),
            UNK_COM2 => todo!("ioctl with com = 0x20006602"),
            UNK_COM3 => todo!("ioctl with com = 0x8004667d"),
            UNK_COM4 => todo!("ioctl with com = 0x8004667e"),
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
        self.revoke(path);

        Ok(SysOut::ZERO)
    }

    /// See `vfs_donmount` on the PS4 for a reference.
    fn mount(
        &self,
        mut opts: HashMap<String, Box<dyn Any>>,
        mut flags: MountFlags,
        td: Option<&VThread>,
    ) -> Result<Arc<Vnode>, MountError> {
        // Process the options.
        let fs = opts.remove("fstype").unwrap().downcast::<String>().unwrap();
        let path = opts
            .remove("fspath")
            .unwrap()
            .downcast::<VPathBuf>()
            .unwrap();

        opts.retain(|k, v| {
            match k.as_str() {
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
                "ro" => flags.set(MountFlags::MNT_RDONLY, *v.downcast_ref::<bool>().unwrap()),
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
                Self::find_config(fs.as_str()).ok_or(MountError::InvalidFs)?
            } else {
                todo!("vfs_donmount with !MNT_ROOTFS");
            };

            // Lookup parent vnode.
            let vn = match self.lookup(path.as_ref(), td) {
                Ok(v) => v,
                Err(e) => return Err(MountError::LookupPathFailed(e)),
            };

            // TODO: Check if jailed.
            let mut mount = Mount::new(
                Some(vn.clone()),
                conf,
                *path,
                td.map_or_else(|| &self.kern, |t| t.cred()),
            );

            flags.remove(MountFlags::from_bits_retain(0xFFFFFFFF272F3F80));
            *mount.flags_mut() = flags;

            // TODO: Implement budgetid.
            if let Err(e) = (mount.fs().ops.mount)(&mut mount, opts) {
                return Err(MountError::MountFailed(e));
            }

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
    fn find_config<N: AsRef<str>>(name: N) -> Option<&'static FsConfig> {
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
    fn to_fflags(self) -> VFileFlags {
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
    ops: &'static FsOps,             // vfc_vfsops
    ty: u32,                         // vfc_typenum
    next: Option<&'static FsConfig>, // vfc_list.next
}

/// An implementation of `vfsops` structure.
#[derive(Debug)]
struct FsOps {
    mount: fn(&mut Mount, HashMap<String, Box<dyn Any>>) -> Result<(), Box<dyn Errno>>,
    root: fn(&Arc<Mount>) -> Arc<Vnode>,
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
pub enum OpenError {}

impl Errno for OpenError {
    fn errno(&self) -> NonZeroI32 {
        todo!()
    }
}

/// Represents an error when [`Fs::lookup()`] was failed.
#[derive(Debug, Error)]
pub enum LookupError {
    #[error("no such file or directory")]
    NotFound,

    #[error("cannot lookup '{1}' from component #{0}")]
    LookupFailed(usize, String, #[source] Box<dyn Errno>),
}

impl Errno for LookupError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::NotFound => ENOENT,
            Self::LookupFailed(_, _, e) => e.errno(),
        }
    }
}

static HOST: FsConfig = FsConfig {
    name: "exfatfs",
    ops: &self::host::HOST_OPS,
    ty: 0x2C,
    next: Some(&MLFS),
};

static MLFS: FsConfig = FsConfig {
    name: "mlfs",
    ops: &MLFS_OPS,
    ty: 0xF1,
    next: Some(&UDF2),
};

static MLFS_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for mlfs"),
    root: |_| todo!("root for mlfs"),
};

static UDF2: FsConfig = FsConfig {
    name: "udf2",
    ops: &UDF2_OPS,
    ty: 0,
    next: Some(&DEVFS),
};

static UDF2_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for udf2"),
    root: |_| todo!("root for udf2"),
};

static DEVFS: FsConfig = FsConfig {
    name: "devfs",
    ops: &self::dev::DEVFS_OPS,
    ty: 0x71,
    next: Some(&TMPFS),
};

static TMPFS: FsConfig = FsConfig {
    name: "tmpfs",
    ops: &self::tmp::TMPFS_OPS,
    ty: 0x87,
    next: Some(&UNIONFS),
};

static UNIONFS: FsConfig = FsConfig {
    name: "unionfs",
    ops: &UNIONFS_OPS,
    ty: 0x41,
    next: Some(&PROCFS),
};

static UNIONFS_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for unionfs"),
    root: |_| todo!("root for unionfs"),
};

static PROCFS: FsConfig = FsConfig {
    name: "procfs",
    ops: &PROCFS_OPS,
    ty: 0x2,
    next: Some(&CD9660),
};

static PROCFS_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for procfs"),
    root: |_| todo!("root for procfs"),
};

static CD9660: FsConfig = FsConfig {
    name: "cd9660",
    ops: &CD9660_OPS,
    ty: 0xBD,
    next: Some(&UFS),
};

static CD9660_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for cd9660"),
    root: |_| todo!("root for cd9660"),
};

static UFS: FsConfig = FsConfig {
    name: "ufs",
    ops: &UFS_OPS,
    ty: 0x35,
    next: Some(&NULLFS),
};

static UFS_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for ufs"),
    root: |_| todo!("root for ufs"),
};

static NULLFS: FsConfig = FsConfig {
    name: "nullfs",
    ops: &NULLFS_OPS,
    ty: 0x29,
    next: Some(&PFS),
};

static NULLFS_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for nullfs"),
    root: |_| todo!("root for nullfs"),
};

static PFS: FsConfig = FsConfig {
    name: "pfs",
    ops: &PFS_OPS,
    ty: 0xA4,
    next: None,
};

static PFS_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for pfs"),
    root: |_| todo!("root for pfs"),
};
