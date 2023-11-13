pub use self::file::*;
pub use self::item::*;
pub use self::path::*;

use crate::errno::{Errno, EBADF, EINVAL, ENOENT, ENOTTY};
use crate::info;
use crate::process::{VProc, VThread};
use crate::syscalls::{SysArg, SysErr, SysIn, SysOut, Syscalls};
use crate::ucred::Privilege;
use bitflags::bitflags;
use gmtx::{GroupMutex, MutexGroup};
use param::Param;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::num::{NonZeroI32, TryFromIntError};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use thiserror::Error;

mod file;
mod item;
mod path;

/// A virtual filesystem for emulating a PS4 filesystem.
#[derive(Debug)]
pub struct Fs {
    vp: Arc<VProc>,
    mounts: GroupMutex<HashMap<VPathBuf, MountSource>>,
    opens: AtomicI32, // openfiles
    app: VPathBuf,
}

//Represents a file descriptor
pub type Fd = i32;

impl Fs {
    pub fn new<S, G>(system: S, game: G, vp: &Arc<VProc>, syscalls: &mut Syscalls) -> Arc<Self>
    where
        S: Into<PathBuf>,
        G: Into<PathBuf>,
    {
        let system = system.into();
        let game = game.into();
        let mut mounts: HashMap<VPathBuf, MountSource> = HashMap::new();

        // Mount rootfs.
        mounts.insert(VPathBuf::new(), MountSource::Host(system.clone()));

        // Get path to param.sfo.
        let mut path = game.join("sce_sys");

        path.push("param.sfo");

        // Open param.sfo.
        let param = match File::open(&path) {
            Ok(v) => v,
            Err(e) => panic!("Cannot open {}: {}.", path.display(), e),
        };

        // Load param.sfo.
        let param = match Param::read(param) {
            Ok(v) => v,
            Err(e) => panic!("Cannot read {}: {}.", path.display(), e),
        };

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
        let mg = MutexGroup::new("fs");
        let app: VPathBuf = format!("/mnt/sandbox/{}_000", param.title_id())
            .try_into()
            .unwrap();

        mounts.insert(app.join("app0").unwrap(), MountSource::Bind(pfs));

        // Install syscall handlers.
        let fs = Arc::new(Self {
            vp: vp.clone(),
            mounts: mg.new_member(mounts),
            opens: AtomicI32::new(0),
            app,
        });

        syscalls.register(4, &fs, Self::sys_write);
        syscalls.register(5, &fs, Self::sys_open);
        syscalls.register(54, &fs, Self::sys_ioctl);
        syscalls.register(56, &fs, Self::sys_revoke);

        fs
    }

    pub fn app(&self) -> &VPath {
        self.app.borrow()
    }

    pub fn get(&self, path: &VPath) -> Result<FsItem, FsError> {
        let item = match path.as_str() {
            "/dev/console" => FsItem::Device(VDev::Console),
            _ => self.resolve(path).ok_or(FsError::NotFound)?,
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

    pub fn revoke<P: Into<VPathBuf>>(&self, _path: P) {
        // TODO: Implement this.
    }

    fn sys_write(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let _fd: Fd = i.args[0].try_into().unwrap();
        let _data: *const u8 = i.args[1].into();
        let _len: usize = i.args[2].try_into().unwrap();

        //TODO: implement this

        Ok(SysOut::ZERO)
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

        info!("Opening {path} with {flags}.");

        // Lookup file.
        *file.flags_mut() = flags.to_fflags();
        file.set_ops(Some(self.get(path)?.open()?));

        // Install to descriptor table.
        let fd = self.vp.files().alloc(Arc::new(file));

        info!("File descriptor {fd} was allocated for {path}.");

        Ok(fd.into())
    }

    fn sys_ioctl(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        const IOC_VOID: u64 = 0x20000000;
        const IOC_OUT: u64 = 0x40000000;
        const IOC_IN: u64 = 0x80000000;
        const IOCPARM_MASK: u64 = 0x1FFF;

        let fd: Fd = i.args[0].try_into().unwrap();
        let mut com: u64 = i.args[1].into();
        let _data: *const u8 = i.args[2].into();

        if com > 0xffffffff {
            com &= 0xffffffff;
        }

        let size = (com >> 16) & IOCPARM_MASK;

        if com & (IOC_VOID | IOC_OUT | IOC_IN) == 0
            || com & (IOC_OUT | IOC_IN) != 0 && size == 0
            || com & IOC_VOID != 0 && size != 0 && size != 4
        {
            return Err(SysErr::Raw(ENOTTY));
        }

        // Get data.
        let data = if size == 0 {
            if com & IOC_IN != 0 {
                todo!("ioctl with IOC_IN");
            } else if com & IOC_OUT != 0 {
                todo!("ioctl with IOC_OUT");
            }

            &[]
        } else {
            todo!("ioctl with size != 0");
        };

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
        let td = VThread::current();

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
            todo!("ioctl with IOC_OUT");
        }

        Ok(SysOut::ZERO)
    }

    fn sys_revoke(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let path = unsafe { i.args[0].to_path()?.unwrap() };

        info!("Revoking access to {path}.");

        // Check current thread privilege.
        VThread::current().priv_check(Privilege::SCE683)?;

        // TODO: Check vnode::v_rdev.
        let file = self.get(path)?;

        if !file.is_character() {
            return Err(SysErr::Raw(EINVAL));
        }

        // TODO: It seems like the initial ucred of the process is either root or has PRIV_VFS_ADMIN
        // privilege.
        self.revoke(path);

        Ok(SysOut::ZERO)
    }

    fn resolve(&self, path: &VPath) -> Option<FsItem> {
        let mounts = self.mounts.read();
        let mut current = VPathBuf::new();
        let root = match mounts.get(&current).unwrap() {
            MountSource::Host(v) => v,
            MountSource::Bind(_) => unreachable!(),
        };

        // Walk on virtual path components.
        let mut directory = HostDir::new(root.clone(), VPathBuf::new());

        for component in path.components() {
            current.push(component).unwrap();

            // Check if a virtual path is a mount point.
            if let Some(mount) = mounts.get(&current) {
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
        self.0.fmt(f)
    }
}

/// Source of mount point.
#[derive(Debug)]
pub enum MountSource {
    Host(PathBuf),
    Bind(VPathBuf),
}

/// Represents an error when the operation of virtual filesystem is failed.
#[derive(Debug, Error)]
pub enum FsError {
    #[error("no such file or directory")]
    NotFound,
}

impl Errno for FsError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::NotFound => ENOENT,
        }
    }
}
