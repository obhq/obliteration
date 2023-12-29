pub use self::cdev::*;
pub use self::dev::*;
pub use self::dirent::*;
pub use self::file::*;
pub use self::host::*;
pub use self::item::*;
pub use self::mount::*;
pub use self::path::*;
pub use self::vnode::*;
use crate::errno::{Errno, EBADF, EINVAL, ENAMETOOLONG, ENODEV, ENOENT, ENOTCAPABLE};
use crate::info;
use crate::process::{VProc, VThread};
use crate::syscalls::{SysArg, SysErr, SysIn, SysOut, Syscalls};
use crate::ucred::{Privilege, Ucred};
use bitflags::bitflags;
use gmtx::{Gutex, GutexGroup};
use param::Param;
use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::num::{NonZeroI32, TryFromIntError};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use thiserror::Error;

mod cdev;
mod dev;
mod dirent;
mod file;
mod host;
mod item;
mod mount;
mod path;
mod vnode;

/// A virtual filesystem for emulating a PS4 filesystem.
#[derive(Debug)]
pub struct Fs {
    vp: Arc<VProc>,
    mounts: Gutex<Mounts>,   // mountlist
    root: Gutex<Arc<Vnode>>, // rootvnode
    opens: AtomicI32,        // openfiles
}

impl Fs {
    pub fn new<S, G>(
        system: S,
        game: G,
        param: &Arc<Param>,
        cred: &Ucred,
        vp: &Arc<VProc>,
        sys: &mut Syscalls,
    ) -> Arc<Self>
    where
        S: Into<PathBuf>,
        G: Into<PathBuf>,
    {
        // Mount devfs as an initial root.
        let mut mounts = Mounts::new();
        let conf = Self::find_config("devfs").unwrap();
        let mut init = Mount::new(None, conf, "/dev", cred.clone());

        if let Err(e) = (init.fs().ops.mount)(&mut init, HashMap::new()) {
            panic!("Failed to mount devfs: {e}.");
        }

        // Get an initial root vnode.
        let root = (init.fs().ops.root)(&mounts.push(init));

        vp.files().set_cwd(root.clone());
        *vp.files().root_mut() = Some(root.clone());

        // Setup mount options for root FS.
        let mut opts: HashMap<String, Box<dyn Any>> = HashMap::new();

        opts.insert("fstype".into(), Box::new(String::from("exfatfs")));
        opts.insert("fspath".into(), Box::new(String::from("/")));
        opts.insert("from".into(), Box::new(String::from("md0")));
        opts.insert("ro".into(), Box::new(true));
        opts.insert("ob:system".into(), Box::new(system.into()));
        opts.insert("ob:game".into(), Box::new(game.into()));
        opts.insert("ob:param".into(), Box::new(param.clone()));

        // Mount root FS.
        let gg = GutexGroup::new();
        let fs = Arc::new(Self {
            vp: vp.clone(),
            mounts: gg.spawn(mounts),
            root: gg.spawn(root),
            opens: AtomicI32::new(0),
        });

        let root = match fs.mount(opts, MountFlags::MNT_ROOTFS, cred) {
            Ok(v) => v,
            Err(e) => panic!("Failed to mount root FS: {e}."),
        };

        // Remove devfs so the root FS become an actual root.
        let om = {
            let mut mounts = fs.mounts.write();
            let old = mounts.remove(0);

            *fs.root.write() = root.clone();

            old
        };

        // Update process location.
        vp.files().set_cwd(root.clone());
        *vp.files().root_mut() = Some(root);

        // Disconnect devfs from the old root.
        *(om.fs().ops.root)(&om).item_mut() = None;

        // Update devfs.
        let mut flags = om.flags_mut();
        let mut parent = om.parent_mut();

        flags.remove(MountFlags::MNT_ROOTFS);
        *parent = None;

        drop(parent);
        drop(flags);

        // TODO: Set devfs parent to /dev on the root FS.
        // Install syscall handlers.
        sys.register(4, &fs, Self::sys_write);
        sys.register(5, &fs, Self::sys_open);
        sys.register(6, &fs, Self::sys_close);
        sys.register(54, &fs, Self::sys_ioctl);
        sys.register(56, &fs, Self::sys_revoke);

        fs
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

    /// See `namei` on the PS4 for a reference.
    pub fn namei(&self, nd: &mut NameiData) -> Result<FsItem, FsError> {
        nd.cnd.cred = nd.cnd.thread.map(|v| v.cred());
        nd.cnd.flags.remove(NameiFlags::TRAILINGSLASH);
        nd.cnd.pnbuf = nd.dirp.as_bytes().to_vec();

        if nd.cnd.flags.intersects(NameiFlags::AUDITVNODE1) {
            // TODO: Implement this.
        }

        if nd.cnd.flags.intersects(NameiFlags::AUDITVNODE2) {
            todo!("namei with AUDITVNODE2");
        }

        if nd.cnd.pnbuf.is_empty() {
            return Err(FsError::NotFound);
        }

        nd.loopcnt = 0;

        // TODO: Implement ktrnamei.
        nd.rootdir = self.vp.files().root().clone();
        nd.topdir = self.vp.files().jail().clone();

        let mut dp = if nd.cnd.pnbuf[0] != b'/' {
            todo!("namei with relative path");
        } else {
            self.vp.files().cwd().clone()
        };

        if nd.startdir.is_some() {
            todo!("namei with ni_startdir");
        }

        // TODO: Implement SDT_PROBE.
        #[allow(clippy::never_loop)] // TODO: Remove this once this loop is fully implemented.
        loop {
            nd.cnd.nameptr = 0;

            if nd.cnd.pnbuf[nd.cnd.nameptr] == b'/' {
                if nd.strictrelative != 0 {
                    return Err(FsError::AbsolutePath);
                }

                loop {
                    nd.cnd.nameptr += 1;

                    if nd.cnd.pnbuf.get(nd.cnd.nameptr).is_some_and(|&v| v != b'/') {
                        break;
                    }
                }

                dp = nd.rootdir.as_ref().unwrap().clone();
            }

            nd.startdir = Some(dp);

            // TODO: Implement the remaining logics from the PS4 when lookup is success.
            // TODO: Implement SDT_PROBE when lookup is failed.
            break self.lookup(nd);
        }
    }

    fn lookup(&self, nd: &mut NameiData) -> Result<FsItem, FsError> {
        nd.cnd.flags.remove(NameiFlags::GIANTHELD);
        nd.cnd.flags.remove(NameiFlags::ISSYMLINK);

        // TODO: Implement the remaining logics from the PS4.
        let item = match nd.dirp {
            "/dev/console" => FsItem::Device(VDev::Console),
            "/dev/dipsw" => FsItem::Device(VDev::Dipsw),
            "/dev/deci_tty6" => FsItem::Device(VDev::DeciTty6),
            "/dev/dmem0" => FsItem::Device(VDev::Dmem0),
            "/dev/dmem1" => FsItem::Device(VDev::Dmem1),
            "/dev/dmem2" => FsItem::Device(VDev::Dmem2),
            _ => {
                let root = self.mounts.read().root().clone();
                let data = root.data().cloned();
                let host = data.unwrap().downcast::<HostFs>().unwrap();

                host.resolve(VPath::new(nd.dirp).unwrap())
                    .ok_or(FsError::NotFound)?
            }
        };

        Ok(item)
    }

    /// See `falloc_noinstall_budget` on the PS4 for a reference.
    pub fn alloc(self: &Arc<Self>) -> VFile {
        // TODO: Check if openfiles exceed rlimit.
        // TODO: Implement budget_resource_use.
        self.opens.fetch_add(1, Ordering::Relaxed);

        VFile::new(self)
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

        let buf = unsafe { std::slice::from_raw_parts(ptr, len) };

        let file = self.vp.files().get(fd).ok_or(SysErr::Raw(EBADF))?;
        let ops = file.ops().ok_or(SysErr::Raw(EBADF))?;

        let td = VThread::current().unwrap();

        info!("Writing {len} bytes to fd {fd}.");

        let bytes_written = ops.write(file.as_ref(), buf, td.cred(), td.as_ref())?;

        Ok(bytes_written.into())
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

        // Allocate file object.
        let mut file = self.alloc();

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
        let mut nd = NameiData {
            dirp: path,
            startdir: None,
            rootdir: None,
            topdir: None,
            strictrelative: 0,
            loopcnt: 0,
            cnd: ComponentName {
                flags: NameiFlags::from_bits_retain(0x5000040),
                thread: Some(&td),
                cred: None,
                pnbuf: Vec::new(),
                nameptr: 0,
            },
        };

        *file.flags_mut() = flags.to_fflags();
        file.set_ops(Some(self.namei(&mut nd)?.open(&self.vp)?));

        // Install to descriptor table.
        let fd = self.vp.files().alloc(Arc::new(file));

        info!("File descriptor {fd} was allocated for {path}.");

        Ok(fd.into())
    }

    fn sys_close(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let fd: i32 = i.args[0].try_into().unwrap();

        info!("Closing fd {fd}.");

        self.vp.files().free(fd)?;

        Ok(SysOut::ZERO)
    }

    const UNK_COM1: IoctlCom = IoctlCom::io(b'f', 1);
    const UNK_COM2: IoctlCom = IoctlCom::io(b'f', 2);
    const UNK_COM3: IoctlCom = IoctlCom::iowint(b'f', 0x7e);
    const UNK_COM4: IoctlCom = IoctlCom::iowint(b'f', 0x7d);

    fn sys_ioctl(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let fd: i32 = i.args[0].try_into().unwrap();
        let com: IoctlCom = i.args[1].try_into()?;
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
        let file = self.vp.files().get(fd).ok_or(SysErr::Raw(EBADF))?;
        let ops = file.ops().ok_or(SysErr::Raw(EBADF))?;

        if !file
            .flags()
            .intersects(VFileFlags::FREAD | VFileFlags::FWRITE)
        {
            return Err(SysErr::Raw(EBADF));
        }

        // Execute the operation.
        let td = VThread::current().unwrap();

        info!("Executing ioctl({com}) on {file}.");

        match com {
            Self::UNK_COM1 => todo!("ioctl with com = 0x20006601"),
            Self::UNK_COM2 => todo!("ioctl with com = 0x20006602"),
            Self::UNK_COM3 => todo!("ioctl with com = 0x8004667d"),
            Self::UNK_COM4 => todo!("ioctl with com = 0x8004667e"),
            _ => {}
        }

        ops.ioctl(&file, com, data, td.cred(), &td)?;

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
        let mut nd = NameiData {
            dirp: path,
            startdir: None,
            rootdir: None,
            topdir: None,
            strictrelative: 0,
            loopcnt: 0,
            cnd: ComponentName {
                flags: NameiFlags::from_bits_retain(0x5000044),
                thread: Some(&td),
                cred: None,
                pnbuf: Vec::new(),
                nameptr: 0,
            },
        };

        let file = self.namei(&mut nd)?;

        if !file.is_character() {
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
        cred: &Ucred,
    ) -> Result<Arc<Vnode>, MountError> {
        // Process the options.
        let fs = opts.remove("fstype").unwrap().downcast::<String>().unwrap();
        let path = opts.remove("fspath").unwrap().downcast::<String>().unwrap();

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

            // TODO: Check if jailed.
            // TODO: Lookup parent vnode.
            let mut mount = Mount::new(None, conf, *path, cred.clone());

            flags.remove(MountFlags::from_bits_retain(0xFFFFFFFF272F3F80));
            *mount.flags_mut() = flags;

            // TODO: Implement budgetid.
            if let Err(e) = (mount.fs().ops.mount)(&mut mount, opts) {
                return Err(MountError::MountFailed(e));
            }

            // TODO: Implement the remaining logics from the PS4.
            Ok((mount.fs().ops.root)(&self.mounts.write().push(mount)))
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

/// An implementation of `nameidata`.
pub struct NameiData<'a> {
    pub dirp: &'a str,                // ni_dirp
    pub startdir: Option<Arc<Vnode>>, // ni_startdir
    pub rootdir: Option<Arc<Vnode>>,  // ni_rootdir
    pub topdir: Option<Arc<Vnode>>,   // ni_topdir
    pub strictrelative: i32,          // ni_strictrelative
    pub loopcnt: u32,                 // ni_loopcnt
    pub cnd: ComponentName<'a>,       // ni_cnd
}

/// An implementation of `componentname`.
pub struct ComponentName<'a> {
    pub flags: NameiFlags,           // cn_flags
    pub thread: Option<&'a VThread>, // cn_thread
    pub cred: Option<&'a Ucred>,     // cn_cred
    pub pnbuf: Vec<u8>,              // cn_pnbuf
    pub nameptr: usize,              // cn_nameptr
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct NameiFlags: u64 {
        const HASBUF = 0x00000400;
        const ISSYMLINK = 0x00010000;
        const GIANTHELD = 0x02000000;
        const AUDITVNODE1 = 0x04000000;
        const AUDITVNODE2 = 0x08000000;
        const TRAILINGSLASH = 0x10000000;
    }
}

bitflags! {
    /// Flags for [`Fs::sys_open()`].
    struct OpenFlags: u32 {
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

/// Represents an error when FS mounting is failed.
#[derive(Debug, Error)]
pub enum MountError {
    #[error("fstype is too long")]
    FsTooLong,

    #[error("fspath is too long")]
    PathTooLong,

    #[error("fstype is not valid")]
    InvalidFs,

    #[error("cannot mount the filesystem")]
    MountFailed(#[source] Box<dyn Errno>),
}

impl Errno for MountError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::FsTooLong | Self::PathTooLong => ENAMETOOLONG,
            Self::InvalidFs => ENODEV,
            Self::MountFailed(e) => e.errno(),
        }
    }
}

/// Represents an error when the operation of virtual filesystem is failed.
#[derive(Debug, Error)]
pub enum FsError {
    #[error("no such file or directory")]
    NotFound,

    #[error("path is absolute")]
    AbsolutePath,
}

impl Errno for FsError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::NotFound => ENOENT,
            Self::AbsolutePath => ENOTCAPABLE,
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
    ops: &TMPFS_OPS,
    ty: 0x87,
    next: Some(&UNIONFS),
};

static TMPFS_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for tmpfs"),
    root: |_| todo!("root for tmpfs"),
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
