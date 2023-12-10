pub use self::file::*;
pub use self::host::*;
pub use self::item::*;
pub use self::mount::*;
pub use self::path::*;
pub use self::vnode::*;
use crate::errno::{Errno, EBADF, EINVAL, ENOENT, ENOTCAPABLE, ENOTTY};
use crate::info;
use crate::process::{VProc, VThread};
use crate::syscalls::{SysArg, SysErr, SysIn, SysOut, Syscalls};
use crate::ucred::{Privilege, Ucred};
use bitflags::bitflags;
use param::Param;
use std::any::Any;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::num::{NonZeroI32, TryFromIntError};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, RwLock};
use thiserror::Error;

mod dev;
#[macro_use]
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
    mounts: RwLock<Vec<Mount>>, // mountlist
    opens: AtomicI32,           // openfiles
    root: Arc<Vnode>,           // rootvnode
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
        let mut mounts = Vec::new();

        // TODO: It seems like the PS4 will mount devfs as an initial rootfs. See vfs_mountroot for
        // more details.
        let mut root = Mount::new(&HOST, cred.clone());
        let mut opts: HashMap<String, Box<dyn Any>> = HashMap::new();

        opts.insert("system".into(), Box::new(system.into()));
        opts.insert("game".into(), Box::new(game.into()));
        opts.insert("param".into(), Box::new(param.clone()));

        if let Err(e) = (root.vfs().ops.mount)(&mut root, opts) {
            panic!("Cannot mount rootfs: {e}.");
        }

        mounts.push(root);

        // Install syscall handlers.
        let fs = Arc::new(Self {
            vp: vp.clone(),
            mounts: RwLock::new(mounts),
            opens: AtomicI32::new(0),
            root: Arc::new(Vnode::new()), // TODO: Check how this constructed on the PS4.
        });

        sys.register(4, &fs, Self::sys_write);
        sys.register(5, &fs, Self::sys_open);
        sys.register(6, &fs, Self::sys_close);
        sys.register(54, &fs, Self::sys_ioctl);
        sys.register(56, &fs, Self::sys_revoke);

        fs
    }

    pub fn app(&self) -> Arc<VPathBuf> {
        let mounts = self.mounts.read().unwrap();
        let root = mounts.first().unwrap();
        let host = root.data().unwrap().downcast_ref::<HostFs>().unwrap();

        host.app().clone()
    }

    pub fn root(&self) -> &Arc<Vnode> {
        &self.root
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
                let mounts = self.mounts.read().unwrap();
                let root = mounts.first().unwrap();
                let host = root.data().unwrap().downcast_ref::<HostFs>().unwrap();

                host.resolve(VPath::new(nd.dirp).unwrap())
                    .ok_or(FsError::NotFound)?
            }
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
    version: u32,                    // vfc_version
    name: &'static str,              // vfc_name
    ops: &'static FsOps,             // vfc_vfsops
    ty: i32,                         // vfc_typenum
    refcount: AtomicI32,             // vfc_refcount
    next: Option<&'static FsConfig>, // vfc_list.next
}

/// An implementation of `vfsops` structure.
#[derive(Debug)]
struct FsOps {
    mount: fn(&mut Mount, HashMap<String, Box<dyn Any>>) -> Result<(), Box<dyn Error>>,
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
    version: 0x19660120,
    name: "exfatfs", // TODO: Seems like the PS4 use exfat as a root FS.
    ops: &self::host::HOST_OPS,
    ty: 0x2C,
    refcount: AtomicI32::new(0),
    next: Some(&MLFS),
};

static MLFS: FsConfig = FsConfig {
    version: 0x19660120,
    name: "mlfs",
    ops: &MLFS_OPS,
    ty: 0xF1,
    refcount: AtomicI32::new(0),
    next: Some(&UDF2),
};

static MLFS_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for mlfs"),
};

static UDF2: FsConfig = FsConfig {
    version: 0x19660120,
    name: "udf2",
    ops: &UDF2_OPS,
    ty: 0,
    refcount: AtomicI32::new(0),
    next: Some(&DEVFS),
};

static UDF2_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for udf2"),
};

static DEVFS: FsConfig = FsConfig {
    version: 0x19660120,
    name: "devfs",
    ops: &DEVFS_OPS,
    ty: 0x71,
    refcount: AtomicI32::new(0),
    next: Some(&TMPFS),
};

static DEVFS_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for devfs"),
};

static TMPFS: FsConfig = FsConfig {
    version: 0x19660120,
    name: "tmpfs",
    ops: &TMPFS_OPS,
    ty: 0x87,
    refcount: AtomicI32::new(0),
    next: Some(&UNIONFS),
};

static TMPFS_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for tmpfs"),
};

static UNIONFS: FsConfig = FsConfig {
    version: 0x19660120,
    name: "unionfs",
    ops: &UNIONFS_OPS,
    ty: 0x41,
    refcount: AtomicI32::new(0),
    next: Some(&PROCFS),
};

static UNIONFS_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for unionfs"),
};

static PROCFS: FsConfig = FsConfig {
    version: 0x19660120,
    name: "procfs",
    ops: &PROCFS_OPS,
    ty: 0x2,
    refcount: AtomicI32::new(0),
    next: Some(&CD9660),
};

static PROCFS_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for procfs"),
};

static CD9660: FsConfig = FsConfig {
    version: 0x19660120,
    name: "cd9660",
    ops: &CD9660_OPS,
    ty: 0xBD,
    refcount: AtomicI32::new(0),
    next: Some(&UFS),
};

static CD9660_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for cd9660"),
};

static UFS: FsConfig = FsConfig {
    version: 0x19660120,
    name: "ufs",
    ops: &UFS_OPS,
    ty: 0x35,
    refcount: AtomicI32::new(0),
    next: Some(&NULLFS),
};

static UFS_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for ufs"),
};

static NULLFS: FsConfig = FsConfig {
    version: 0x19660120,
    name: "nullfs",
    ops: &NULLFS_OPS,
    ty: 0x29,
    refcount: AtomicI32::new(0),
    next: Some(&PFS),
};

static NULLFS_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for nullfs"),
};

static PFS: FsConfig = FsConfig {
    version: 0x19660120,
    name: "pfs",
    ops: &PFS_OPS,
    ty: 0xA4,
    refcount: AtomicI32::new(0),
    next: None,
};

static PFS_OPS: FsOps = FsOps {
    mount: |_, _| todo!("mount for pfs"),
};
