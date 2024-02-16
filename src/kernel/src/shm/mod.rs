use crate::{
    errno::{Errno, EINVAL},
    fs::{
        check_access, Access, DefaultError, FileBackend, Mode, OpenFlags, Stat, VFile, VFileFlags,
        VPathBuf,
    },
    memory::MemoryManager,
    process::VThread,
    syscalls::{SysErr, SysIn, SysOut, Syscalls},
    ucred::{Gid, Ucred, Uid},
};
use std::{
    collections::HashMap,
    convert::Infallible,
    sync::{Arc, RwLock},
};

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

        let fd = filedesc.alloc_without_budget::<Infallible>(
            |_| match path {
                ShmPath::Anon => {
                    todo!()
                }
                ShmPath::Path(path) => {
                    todo!()
                }
            },
            (flags & OpenFlags::O_ACCMODE).into_fflags(),
        )?;

        Ok(fd.into())
    }

    fn sys_shm_unlink(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        todo!("sys_shm_unlink")
    }
}

pub enum ShmPath {
    Anon,
    Path(VPathBuf),
}

#[derive(Debug)]
pub struct Shm {
    uid: Uid,
    gid: Gid,
    mode: Mode,
}

impl Shm {
    /// See `shm_do_truncate` on the PS4 for a reference.
    fn truncate(&self, size: usize) {
        todo!()
    }

    /// See `shm_access` on the PS4 for a reference.
    fn access(&self, cred: &Ucred, flags: VFileFlags) -> Result<(), Box<dyn Errno>> {
        let mut access = Access::empty();

        if flags.intersects(VFileFlags::READ) {
            access |= Access::READ;
        }

        if flags.intersects(VFileFlags::WRITE) {
            access |= Access::WRITE;
        }

        check_access(cred, self.uid, self.gid, self.mode, access, false)?;

        Ok(())
    }
}

impl FileBackend for Shm {
    #[allow(unused_variables)]
    fn read(
        self: &Arc<Self>,
        file: &VFile,
        buf: &mut crate::fs::UioMut,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        Err(DefaultError::OperationNotSupported.into())
    }

    #[allow(unused_variables)]
    fn write(
        self: &Arc<Self>,
        file: &VFile,
        buf: &mut crate::fs::Uio,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        Err(DefaultError::OperationNotSupported.into())
    }

    #[allow(unused_variables)] // remove when implementing
    fn ioctl(
        self: &Arc<Self>,
        file: &VFile,
        cmd: crate::fs::IoCmd,
        data: &mut [u8],
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // remove when implementing
    fn stat(self: &Arc<Self>, file: &VFile, td: Option<&VThread>) -> Result<Stat, Box<dyn Errno>> {
        let mut stat = Stat::zeroed();

        stat.block_size = 0x4000;

        todo!()
    }
}
