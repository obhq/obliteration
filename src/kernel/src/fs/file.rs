use super::{IoCmd, Offset, Stat, Uio, UioMut, Vnode};
use crate::errno::Errno;
use crate::process::VThread;
use bitflags::bitflags;
use std::any::Any;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::Arc;

/// An implementation of `file` structure.
#[derive(Debug)]
pub struct VFile {
    ty: VFileType,                    // f_type
    data: Arc<dyn Any + Send + Sync>, // f_data
    ops: &'static VFileOps,           // f_ops
    flags: VFileFlags,                // f_flag
}

impl VFile {
    pub(super) fn new(
        ty: VFileType,
        data: Arc<dyn Any + Send + Sync>,
        ops: &'static VFileOps,
    ) -> Self {
        Self {
            ty,
            data,
            ops,
            flags: VFileFlags::empty(),
        }
    }

    pub fn flags(&self) -> VFileFlags {
        self.flags
    }

    pub fn flags_mut(&mut self) -> &mut VFileFlags {
        &mut self.flags
    }

    pub fn ops(&self) -> &'static VFileOps {
        self.ops
    }

    /// See `dofileread` on the PS4 for a reference.
    pub fn do_read(
        &self,
        uio: UioMut,
        off: Offset,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        if uio.bytes_left == 0 {
            return Ok(0);
        }

        // TODO: consider implementing ktrace.

        let res = self.read(uio, off, td);

        if let Err(ref e) = res {
            todo!()
        }

        res
    }

    /// See `fo_read` on the PS4 for a reference.
    fn read(
        &self,
        mut uio: UioMut,
        off: Offset,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        (self.ops.read)(self, &mut uio, off, td)
    }

    /// See `dofilewrite` on the PS4 for a reference.
    pub fn do_write(
        &self,
        mut uio: Uio,
        off: Offset,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        // TODO: consider implementing ktrace.
        // TODO: implement bwillwrite.

        let res = self.write(&mut uio, off, td);

        if let Err(ref e) = res {
            todo!()
        }

        res
    }

    /// See `fo_write` on the PS4 for a reference.
    fn write(
        &self,
        uio: &mut Uio,
        off: Offset,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        (self.ops.write)(self, uio, off, td)
    }

    /// See `fo_ioctl` on the PS4 for a reference.
    pub fn ioctl(
        &self,
        cmd: IoCmd,
        data: &mut [u8],
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        (self.ops.ioctl)(self, cmd, data, td)
    }

    pub fn stat(&self, td: Option<&VThread>) -> Result<Stat, Box<dyn Errno>> {
        (self.ops.stat)(self, td)
    }
}

impl Seek for VFile {
    fn seek(&mut self, _pos: SeekFrom) -> std::io::Result<u64> {
        todo!()
    }
}

impl Read for VFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        todo!()
    }
}

impl Write for VFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> std::io::Result<()> {
        todo!()
    }
}

/// Type of [`VFile`].
#[derive(Debug)]
#[rustfmt::skip]
pub enum VFileType {
    Vnode(Arc<Vnode>), // DTYPE_VNODE = 1
//  Socket,            // DTYPE_SOCKET = 2
//  Pipe,              // DTYPE_PIPE = 3
//  Fifo,              // DTYPE_FIFO = 4
//  Kqueue,            // DTYPE_KQUEUE = 5s
//  Crypto,            // DTYPE_CRYPTO = 6 (crypto device)
//  Mqueue,            // DTYPE_MQUEUE = 7 (POSIX message queues)
//  Shm,               // DTYPE_SHM = 8 (POSIX shared memory)
//  Sem,               // DTYPE_SEM = 9 (POSIX semaphores)
//  Pts,               // DTYPE_PTS = 10 (pseudo teletype master device)
//  Dev,               // DTYPE_DEV = 11
//  Cap,               // DTYPE_CAPABILITY = 12 (capability)
//  ProcDesc,          // DTYPE_PROCDESC = 13 (process descriptor)
//  JitShm,            // DTYPE_JITSHM = 14 (JIT shared memory)
//  IpcSocket,         // DTYPE_IPCSOCKET = 15
//  Physhm,            // DTYPE_PHYSHM = 16 (physical shared memory)
//  Blockpool,         // DTYPE_BLOCKPOOL = 17
}

/// An implementation of `fileops` structure.
#[derive(Debug)]
pub struct VFileOps {
    pub read: VFileRead,
    pub write: VFileWrite,
    pub ioctl: VFileIoctl,
    pub stat: VFileStat,
    pub flags: VFileOpsFlags,
}

impl VFileOps {
    pub fn flags(&self) -> VFileOpsFlags {
        self.flags
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct VFileOpsFlags: u32 {
        const PASSABLE = 0x00000001; // DFLAG_PASSABLE
        const SEEKABLE = 0x00000002; // DFLAG_SEEKABLE
    }
}

type VFileRead = fn(&VFile, &mut UioMut, Offset, Option<&VThread>) -> Result<usize, Box<dyn Errno>>;
type VFileWrite = fn(&VFile, &mut Uio, Offset, Option<&VThread>) -> Result<usize, Box<dyn Errno>>;
type VFileIoctl = fn(&VFile, IoCmd, &mut [u8], Option<&VThread>) -> Result<(), Box<dyn Errno>>;
type VFileStat = fn(&VFile, Option<&VThread>) -> Result<Stat, Box<dyn Errno>>;

bitflags! {
    /// Flags for [`VFile`].
    #[derive(Debug, Clone, Copy)]
    pub struct VFileFlags: u32 {
        const READ = 0x00000001; // FREAD
        const WRITE = 0x00000002; // FWRITE
    }
}
