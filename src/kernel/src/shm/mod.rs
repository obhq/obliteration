use crate::{
    errno::{Errno, EINVAL},
    fs::{
        check_access, Access, AccessError, DefaultFileBackendError, FileBackend, IoCmd, Mode,
        OpenFlags, PollEvents, Stat, TruncateLength, Uio, UioMut, VFile, VFileFlags, VPathBuf,
    },
    process::VThread,
    syscalls::{SysErr, SysIn, SysOut, Syscalls},
    ucred::{Gid, Ucred, Uid},
};
use macros::Errno;
use std::{convert::Infallible, sync::Arc};
use thiserror::Error;

pub struct SharedMemoryManager {}

impl SharedMemoryManager {
    pub fn new(sys: &mut Syscalls) -> Arc<Self> {
        let shm = Arc::new(Self {});

        sys.register(482, &shm, Self::sys_shm_open);
        sys.register(483, &shm, Self::sys_shm_unlink);

        shm
    }

    fn sys_shm_open(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
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

        let filedesc = td.proc().files();

        #[allow(unused_variables)] // TODO: remove when implementing.
        let mode = mode & filedesc.cmask() & 0o7777;

        let fd = filedesc.alloc_without_budget::<Infallible>(
            |_| match path {
                ShmPath::Anon => {
                    todo!()
                }
                ShmPath::Path(_) => {
                    todo!()
                }
            },
            (flags & OpenFlags::O_ACCMODE).into_fflags(),
        )?;

        Ok(fd.into())
    }

    #[allow(unused_variables)] // TODO: remove when implementing.
    fn sys_shm_unlink(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        todo!("sys_shm_unlink")
    }
}

pub enum ShmPath {
    Anon,
    Path(VPathBuf),
}

/// An implementation of the `shmfd` structure.
#[derive(Debug)]
#[allow(unused_variables)] // TODO: remove when used.
pub struct SharedMemory {
    uid: Uid,
    gid: Gid,
    mode: Mode,
}

impl SharedMemory {
    /// See `shm_do_truncate` on the PS4 for a reference.
    #[allow(unused_variables)] // TODO: remove when implementing.
    fn do_truncate(&self, length: TruncateLength) -> Result<(), TruncateError> {
        todo!()
    }

    /// See `shm_access` on the PS4 for a reference.
    #[allow(dead_code)] // TODO: remove when used.
    fn access(&self, cred: &Ucred, flags: VFileFlags) -> Result<(), AccessError> {
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

impl FileBackend for SharedMemory {
    fn read(
        self: &Arc<Self>,
        _: &VFile,
        _: &mut UioMut,
        _: i64,
        _: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        Err(Box::new(DefaultFileBackendError::OperationNotSupported))
    }

    fn write(
        self: &Arc<Self>,
        _: &VFile,
        _: &mut Uio,
        _: i64,
        _: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        Err(Box::new(DefaultFileBackendError::OperationNotSupported))
    }

    #[allow(unused_variables)] // remove when implementing
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

    #[allow(unused_variables)] // remove when implementing
    fn stat(self: &Arc<Self>, file: &VFile, td: Option<&VThread>) -> Result<Stat, Box<dyn Errno>> {
        let mut stat = Stat::zeroed();

        stat.block_size = 0x4000;

        todo!()
    }

    fn truncate(
        self: &Arc<Self>,
        _: &VFile,
        length: TruncateLength,
        _: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        self.do_truncate(length)?;

        Ok(())
    }
}

#[derive(Debug, Error, Errno)]
pub enum TruncateError {}
