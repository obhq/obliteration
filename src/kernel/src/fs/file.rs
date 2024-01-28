use super::{IoCmd, Stat, Vnode};
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

    pub fn write(&self, data: &[u8], td: Option<&VThread>) -> Result<usize, Box<dyn Errno>> {
        (self.ops.write)(self, data, td)
    }

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
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
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
//  Kqueue,            // DTYPE_KQUEUE = 5
//  Crypto,            // DTYPE_CRYPTO = 6
//  Mqueue,            // DTYPE_MQUEUE = 7 (POSIX message queues)
//  Shm,               // DTYPE_SHM = 8 (POSIX shared memory)
//  Sem,               // DTYPE_SEM = 9 (POSIX semaphores)
//  Pts,               // DTYPE_PTS = 10
//  Dev,               // DTYPE_DEV = 11
//  Cap,               // DTYPE_CAPABILITY = 12
//  ProcDesc,          // DTYPE_PROCDESC = 13
//  JitShm,            // (presumably) DTYPE_JITSHM = 14
//  Socket2,           // unknown = 15 // figure out what exactly this is
//  Physhm,            // (presumably) DTYPE_PHYSHM = 16
//  Blockpool,         // (presumably) DTYPE_BLOCKPOOL = 17
}

/// An implementation of `fileops` structure.
#[derive(Debug)]
pub struct VFileOps {
    pub read: VFileRead,
    pub write: VFileWrite,
    pub ioctl: VFileIoctl,
    pub stat: VFileStat,
}

type VFileRead = fn(&VFile, &mut [u8], Option<&VThread>) -> Result<usize, Box<dyn Errno>>;
type VFileWrite = fn(&VFile, &[u8], Option<&VThread>) -> Result<usize, Box<dyn Errno>>;
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
