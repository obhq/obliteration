use crate::{
    errno::{Errno, EEXIST, EINVAL, ENOTTY, EOPNOTSUPP},
    fs::{check_access, Access, IoCmd, Mode, OpenFlags, VFile, VFileFlags, VFileOps, VPathBuf},
    memory::MemoryManager,
    process::VThread,
    syscalls::{SysErr, SysIn, SysOut, Syscalls},
    ucred::{Gid, Ucred, Uid},
};
use macros::vpath;
use std::{
    collections::HashMap,
    num::NonZeroI32,
    sync::{Arc, RwLock},
};
use thiserror::Error;

pub struct SharedMemoryManager {
    mm: Arc<MemoryManager>,
    map: RwLock<HashMap<VPathBuf, Arc<Shm>>>,
}

impl SharedMemoryManager {
    pub fn new(mm: &Arc<MemoryManager>, sys: &mut Syscalls) -> Arc<Self> {
        let shm = Arc::new(Self {
            mm: mm.clone(),
            map: RwLock::default(),
        });

        sys.register(482, &shm, Self::sys_shm_open);
        sys.register(483, &shm, Self::sys_shm_unlink);

        shm
    }

    fn sys_shm_open(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let path = unsafe { i.args[0].to_shm_path() }?.expect("invalid shm path");
        let flags: OpenFlags = i.args[1].try_into().unwrap();
        let mode: u32 = i.args[2].try_into().unwrap();

        if (flags & OpenFlags::O_ACCMODE != OpenFlags::O_RDONLY)
            || (flags & OpenFlags::O_ACCMODE != OpenFlags::O_RDWR)
        {
            return Err(SysErr::Raw(EINVAL));
        }

        if !flags
            .difference(
                OpenFlags::O_ACCMODE | OpenFlags::O_CREAT | OpenFlags::O_EXCL | OpenFlags::O_TRUNC,
            )
            .is_empty()
        {
            return Err(SysErr::Raw(EINVAL));
        }

        let td = VThread::current().unwrap();

        let filedesc = td.proc().files();

        let mode = mode & filedesc.cmask() & 0o7777;

        let fd = td.falloc(
            (flags & OpenFlags::O_ACCMODE).into_fflags(),
            &SHM_FILEOPS,
            |_| match path {
                ShmPath::Anon => {
                    if flags & OpenFlags::O_ACCMODE == OpenFlags::O_RDONLY {
                        return Err(SysErr::Raw(EINVAL));
                    }

                    Ok(self.alloc_shm(td.cred(), mode))
                }
                ShmPath::Path(path) => {
                    let path = path.deref();
                    if path != vpath!("/SceWebCore") {
                        return Err(SysErr::Raw(EINVAL));
                    }

                    let mut map = self.map.write().expect("lock poisoned");

                    if let Some(shm) = map.get(path) {
                        if flags
                            .difference(OpenFlags::O_CREAT | OpenFlags::O_EXCL)
                            .is_empty()
                        {
                            return Err(SysErr::Raw(EEXIST));
                        }

                        shm.access(td.cred(), (flags & OpenFlags::O_ACCMODE).into_fflags())?;

                        if flags.difference(OpenFlags::O_ACCMODE | OpenFlags::O_TRUNC)
                            == OpenFlags::O_RDWR | OpenFlags::O_TRUNC
                        {
                            shm.truncate(0);
                        }

                        Ok(shm.clone())
                    } else {
                        if !flags.intersects(OpenFlags::O_CREAT) {
                            return Err(SysErr::Raw(EINVAL));
                        }

                        let shm = self.alloc_shm(td.cred(), mode);

                        map.insert(path.to_owned(), shm.clone());

                        Ok(shm)
                    }
                }
            },
        )?;

        Ok(fd.into())
    }

    fn alloc_shm(&self, cred: &Ucred, mode: u32) -> Arc<Shm> {
        todo!()
    }

    fn sys_shm_unlink(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        todo!("sys_shm_unlink")
    }
}

pub enum ShmPath {
    Anon,
    Path(VPathBuf),
}

pub struct Shm {
    size: usize, // shm_size
    uid: Uid,    // shm_uid
    gid: Gid,    // shm_gid
    mode: Mode,  // shm_mode
}

impl Shm {
    /// See `shm_do_truncate` on the PS4 for a reference.
    fn truncate(&self, size: usize) {
        todo!()
    }

    /// See `shm_access` on the PS4 for a reference.
    fn access(&self, cred: &Ucred, flags: VFileFlags) -> Result<(), Box<dyn Errno>> {
        let mut access = Access::empty();

        if flags.intersects(VFileFlags::FREAD) {
            access |= Access::READ;
        }

        if flags.intersects(VFileFlags::FWRITE) {
            access |= Access::WRITE;
        }

        check_access(cred, self.uid, self.gid, self.mode, access, false)?;

        Ok(())
    }
}

static SHM_FILEOPS: VFileOps = VFileOps {
    read: |_, _, _| Err(GenericError::NotSupported)?,
    write: |_, _, _| Err(GenericError::NotSupported)?,
    ioctl: shm_ioctl,
};

fn shm_ioctl(
    file: &VFile,
    cmd: IoCmd,
    data: &mut [u8],
    td: Option<&VThread>,
) -> Result<(), Box<dyn Errno>> {
    match cmd {
        IoCmd::SHMCMD0 => todo!(),
        IoCmd::SHMCMD1 => todo!(),
        _ => Err(IoctlError::InvalidCommand)?,
    }

    Ok(())
}

impl IoCmd {
    const SHMCMD0: IoCmd = IoCmd::ior::<i32>(0xa1, 0);
    const SHMCMD1: IoCmd = IoCmd::ior::<i32>(0xa1, 1);
}

#[derive(Debug, Error)]
pub enum GenericError {
    #[error("operation not supported")]
    NotSupported,
}

impl Errno for GenericError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::NotSupported => EOPNOTSUPP,
        }
    }
}

#[derive(Debug, Error)]
pub enum IoctlError {
    #[error("invalid argument")]
    InvalidCommand,
}

impl Errno for IoctlError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::InvalidCommand => ENOTTY,
        }
    }
}
