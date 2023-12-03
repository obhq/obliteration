pub use self::file::*;
pub use self::item::*;
pub use self::path::*;
pub use self::vnode::*;
use crate::errno::{Errno, EBADF, EINVAL, ENOENT, ENOTCAPABLE, ENOTTY};
use crate::info;
use crate::process::{VProc, VThread};
use crate::syscalls::{SysArg, SysErr, SysIn, SysOut, Syscalls};
use crate::ucred::{Privilege, Ucred};
use bitflags::bitflags;
use param::Param;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::num::{NonZeroI32, TryFromIntError};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use thiserror::Error;

mod dev;
mod file;
mod item;
mod path;
mod vnode;

/// A virtual filesystem for emulating a PS4 filesystem.
#[derive(Debug)]
pub struct Fs {
    vp: Arc<VProc>,
    mounts: HashMap<VPathBuf, MountSource>,
    opens: AtomicI32, // openfiles
    app: VPathBuf,
}

impl Fs {
    pub fn new<S, G>(
        system: S,
        game: G,
        param: &Param,
        vp: &Arc<VProc>,
        sys: &mut Syscalls,
    ) -> Arc<Self>
    where
        S: Into<PathBuf>,
        G: Into<PathBuf>,
    {
        let system = system.into();
        let game = game.into();
        let mut mounts: HashMap<VPathBuf, MountSource> = HashMap::new();

        // Mount rootfs.
        mounts.insert(VPathBuf::new(), MountSource::Host(system.clone()));

        // Create a directory for mounting PFS.
        let mut pfs = system.join("mnt");

        pfs.push("sandbox");
        pfs.push("pfsmnt");

        if let Err(e) = std::fs::create_dir_all(&pfs) {
            panic!("Cannot create {}: {}.", pfs.display(), e);
        }

        // Mount game directory.
        let pfs: VPathBuf = format!("/mnt/sandbox/pfsmnt/{}-app0-patch0-union", param.title_id())
            .try_into()
            .unwrap();

        mounts.insert(pfs.clone(), MountSource::Host(game));

        // Create a directory for mounting app0.
        let mut app = system.join("mnt");

        app.push("sandbox");
        app.push(format!("{}_000", param.title_id()));

        if let Err(e) = std::fs::create_dir_all(&app) {
            panic!("Cannot create {}: {}.", app.display(), e);
        }

        // Mount /mnt/sandbox/{id}_000/app0 to /mnt/sandbox/pfsmnt/{id}-app0-patch0-union.
        let app: VPathBuf = format!("/mnt/sandbox/{}_000", param.title_id())
            .try_into()
            .unwrap();

        mounts.insert(app.join("app0").unwrap(), MountSource::Bind(pfs));

        // Install syscall handlers.
        let fs = Arc::new(Self {
            vp: vp.clone(),
            mounts,
            opens: AtomicI32::new(0),
            app,
        });

        sys.register(4, &fs, Self::sys_write);
        sys.register(5, &fs, Self::sys_open);
        sys.register(6, &fs, Self::sys_close);
        sys.register(54, &fs, Self::sys_ioctl);
        sys.register(56, &fs, Self::sys_revoke);

        fs
    }

    pub fn app(&self) -> &VPath {
        self.app.borrow()
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
        nd.rootdir = Some(self.vp.files().root().clone());
        nd.topdir = Some(self.vp.files().jail().clone());

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

                    if nd
                        .cnd
                        .pnbuf
                        .get(nd.cnd.nameptr)
                        .filter(|&v| *v == b'/')
                        .is_none()
                    {
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
        // TODO: Implement logics from the PS4.
        let item = match nd.dirp {
            "/dev/console" => FsItem::Device(VDev::Console),
            "/dev/dipsw" => FsItem::Device(VDev::Dipsw),
            "/dev/deci_tty6" => FsItem::Device(VDev::DeciTty6),
            _ => self
                .resolve(VPath::new(nd.dirp).unwrap())
                .ok_or(FsError::NotFound)?,
        };

        Ok(item)
    }

    /// See `falloc_noinstall_budget` on the PS4 for a reference.
    fn alloc(self: &Arc<Self>) -> VFile {
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
        file.set_ops(Some(self.namei(&mut nd)?.open()?));

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

    fn sys_ioctl(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        const IOC_VOID: u64 = 0x20000000;
        const IOC_OUT: u64 = 0x40000000;
        const IOC_IN: u64 = 0x80000000;
        const IOCPARM_MASK: u64 = 0x1FFF;

        let fd: i32 = i.args[0].try_into().unwrap();
        let mut com: u64 = i.args[1].into();
        let data_arg: *mut u8 = i.args[2].into();

        if com > 0xffffffff {
            com &= 0xffffffff;
        }

        let size: usize = ((com >> 16) & IOCPARM_MASK) as usize;

        if com & (IOC_VOID | IOC_OUT | IOC_IN) == 0
            || com & (IOC_OUT | IOC_IN) != 0 && size == 0
            || com & IOC_VOID != 0 && size != 0 && size != 4
        {
            return Err(SysErr::Raw(ENOTTY));
        }

        let mut vec = vec![0u8; size];

        // Get data.
        let data = if size == 0 {
            &mut []
        } else {
            if com & IOC_VOID != 0 {
                todo!("ioctl with com & IOC_VOID != 0");
            } else {
                &mut vec[..]
            }
        };

        if com & IOC_IN != 0 {
            todo!("ioctl with IOC_IN");
        } else if com & IOC_OUT != 0 {
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

        info!("Executing ioctl({com:#x}) on {file}.");

        match com {
            0x20006601 => todo!("ioctl with com = 0x20006601"),
            0x20006602 => todo!("ioctl with com = 0x20006602"),
            0x8004667d => todo!("ioctl with com = 0x8004667d"),
            0x8004667e => todo!("ioctl with com = 0x8004667e"),
            _ => {}
        }

        ops.ioctl(&file, com, data, td.cred(), &td)?;

        if com & IOC_OUT != 0 {
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

    fn resolve(&self, path: &VPath) -> Option<FsItem> {
        let mut current = VPathBuf::new();
        let root = match self.mounts.get(&current).unwrap() {
            MountSource::Host(v) => v,
            MountSource::Bind(_) => unreachable!(),
        };

        // Walk on virtual path components.
        let mut directory = HostDir::new(root.clone(), VPathBuf::new());

        for component in path.components() {
            current.push(component).unwrap();

            // Check if a virtual path is a mount point.
            if let Some(mount) = self.mounts.get(&current) {
                let path = match mount {
                    MountSource::Host(v) => v.to_owned(),
                    MountSource::Bind(v) => match self.resolve(v)? {
                        FsItem::Directory(d) => d.into_path(),
                        _ => unreachable!(),
                    },
                };

                directory = HostDir::new(path, VPathBuf::new());
            } else {
                // Build a real path.
                let mut path = directory.into_path();

                path.push(component);

                // Get file metadata.
                let meta = match std::fs::metadata(&path) {
                    Ok(v) => v,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::NotFound {
                            return None;
                        } else {
                            panic!("Cannot get the metadata of {}: {e}.", path.display());
                        }
                    }
                };

                // Check file type.
                if meta.is_file() {
                    return Some(FsItem::File(HostFile::new(path, current)));
                }

                directory = HostDir::new(path, VPathBuf::new());
            }
        }

        // If we reached here that mean the the last component is a directory.
        Some(FsItem::Directory(HostDir::new(
            directory.into_path(),
            current,
        )))
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

/// Source of mount point.
#[derive(Debug)]
enum MountSource {
    Host(PathBuf),
    Bind(VPathBuf),
}

/// An implementation of `vfsconf` structure.
struct FsConfig {
    version: u32,                    // vfc_version
    name: &'static str,              // vfc_name
    ops: &'static FsOps,             // vfc_vfsops
    ty: i32,                         // vfc_typenum
    next: Option<&'static FsConfig>, // vfc_list_next
}

/// An implementation of `vfsops` structure.
struct FsOps {}

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

static CONFIGS: &'static FsConfig = &EXFAT;

static EXFAT: FsConfig = FsConfig {
    version: 0x19660120,
    name: "exfatfs",
    ops: &EXFAT_OPS,
    ty: 0x2C,
    next: Some(&MLFS),
};

static EXFAT_OPS: FsOps = FsOps {};

static MLFS: FsConfig = FsConfig {
    version: 0x19660120,
    name: "mlfs",
    ops: &MLFS_OPS,
    ty: 0xF1,
    next: Some(&UDF2),
};

static MLFS_OPS: FsOps = FsOps {};

static UDF2: FsConfig = FsConfig {
    version: 0x19660120,
    name: "udf2",
    ops: &UDF2_OPS,
    ty: 0,
    next: Some(&DEVFS),
};

static UDF2_OPS: FsOps = FsOps {};

static DEVFS: FsConfig = FsConfig {
    version: 0x19660120,
    name: "devfs",
    ops: &DEVFS_OPS,
    ty: 0x71,
    next: Some(&TMPFS),
};

static DEVFS_OPS: FsOps = FsOps {};

static TMPFS: FsConfig = FsConfig {
    version: 0x19660120,
    name: "tmpfs",
    ops: &TMPFS_OPS,
    ty: 0x87,
    next: Some(&UNIONFS),
};

static TMPFS_OPS: FsOps = FsOps {};

static UNIONFS: FsConfig = FsConfig {
    version: 0x19660120,
    name: "unionfs",
    ops: &UNIONFS_OPS,
    ty: 0x41,
    next: Some(&PROCFS),
};

static UNIONFS_OPS: FsOps = FsOps {};

static PROCFS: FsConfig = FsConfig {
    version: 0x19660120,
    name: "procfs",
    ops: &PROCFS_OPS,
    ty: 0x2,
    next: Some(&CD9660),
};

static PROCFS_OPS: FsOps = FsOps {};

static CD9660: FsConfig = FsConfig {
    version: 0x19660120,
    name: "cd9660",
    ops: &CD9660_OPS,
    ty: 0xBD,
    next: Some(&UFS),
};

static CD9660_OPS: FsOps = FsOps {};

static UFS: FsConfig = FsConfig {
    version: 0x19660120,
    name: "ufs",
    ops: &UFS_OPS,
    ty: 0x35,
    next: Some(&NULLFS),
};

static UFS_OPS: FsOps = FsOps {};

static NULLFS: FsConfig = FsConfig {
    version: 0x19660120,
    name: "nullfs",
    ops: &NULLFS_OPS,
    ty: 0x29,
    next: Some(&PFS),
};

static NULLFS_OPS: FsOps = FsOps {};

static PFS: FsConfig = FsConfig {
    version: 0x19660120,
    name: "pfs",
    ops: &PFS_OPS,
    ty: 0xA4,
    next: None,
};

static PFS_OPS: FsOps = FsOps {};
