use super::{CharacterDevice, IoCmd, Stat, TruncateLength, Uio, UioMut, Vnode};
use crate::dmem::BlockPool;
use crate::errno::Errno;
use crate::errno::{EINVAL, ENOTTY, ENXIO, EOPNOTSUPP};
use crate::fs::{IoVec, PollEvents};
use crate::kqueue::KernelQueue;
use crate::net::Socket;
use crate::process::VThread;
use crate::shm::SharedMemory;
use bitflags::bitflags;
use gmtx::{Gutex, GutexGroup};
use macros::Errno;
use std::fmt::Debug;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::sync::Arc;
use thiserror::Error;

/// An implementation of `file` structure.
#[derive(Debug)]
pub struct VFile {
    ty: VFileType,      // f_type
    flags: VFileFlags,  // f_flag
    offset: Gutex<i64>, // f_offset
}

impl VFile {
    pub fn new(ty: VFileType, flags: VFileFlags) -> Self {
        let gg = GutexGroup::new();

        Self {
            ty,
            flags,
            offset: gg.spawn(0),
        }
    }

    pub fn ty(&self) -> &VFileType {
        &self.ty
    }

    pub fn flags(&self) -> VFileFlags {
        self.flags
    }

    /// Checking if this returns `Some` is equivalent to when FreeBSD and the PS4 check
    /// fp->f_ops->fo_flags & DFLAG_SEEKABLE != 0, therefore we use this instead.
    pub fn seekable_vnode(&self) -> Option<&Arc<Vnode>> {
        match &self.ty {
            VFileType::Vnode(vn) => Some(vn),
            VFileType::Device(_) => todo!(),
            _ => None,
        }
    }

    /// See `dofileread` on the PS4 for a reference.
    pub fn do_read(&self, mut uio: UioMut, td: Option<&VThread>) -> Result<usize, Box<dyn Errno>> {
        if uio.bytes_left == 0 {
            return Ok(0);
        }

        // TODO: consider implementing ktrace.

        todo!()
    }

    /// See `dofilewrite` on the PS4 for a reference.
    pub fn do_write(&self, mut uio: Uio, td: Option<&VThread>) -> Result<usize, Box<dyn Errno>> {
        // TODO: consider implementing ktrace.
        // TODO: implement bwillwrite.

        todo!()
    }

    fn read(&self, buf: &mut UioMut, td: Option<&VThread>) -> Result<(), Box<dyn Errno>> {
        match &self.ty {
            VFileType::Vnode(vn) => FileBackend::read(vn, self, buf, td),
            VFileType::Socket(so) | VFileType::IpcSocket(so) => so.read(self, buf, td),
            VFileType::KernelQueue(kq) => kq.read(self, buf, td),
            VFileType::SharedMemory(shm) => shm.read(self, buf, td),
            VFileType::Device(dev) => dev.read(self, buf, td),
            VFileType::Blockpool(bp) => bp.read(self, buf, td),
        }
    }

    fn write(&self, buf: &mut Uio, td: Option<&VThread>) -> Result<(), Box<dyn Errno>> {
        match &self.ty {
            VFileType::Vnode(vn) => FileBackend::write(vn, self, buf, td),
            VFileType::Socket(so) | VFileType::IpcSocket(so) => so.write(self, buf, td),
            VFileType::KernelQueue(kq) => kq.write(self, buf, td),
            VFileType::SharedMemory(shm) => shm.write(self, buf, td),
            VFileType::Device(dev) => dev.write(self, buf, td),
            VFileType::Blockpool(bp) => bp.write(self, buf, td),
        }
    }

    /// See `fo_ioctl` on the PS4 for a reference.
    pub fn ioctl(&self, cmd: IoCmd, td: Option<&VThread>) -> Result<(), Box<dyn Errno>> {
        match &self.ty {
            VFileType::Vnode(vn) => vn.ioctl(self, cmd, td),
            VFileType::Socket(so) | VFileType::IpcSocket(so) => so.ioctl(self, cmd, td),
            VFileType::KernelQueue(kq) => kq.ioctl(self, cmd, td),
            VFileType::SharedMemory(shm) => shm.ioctl(self, cmd, td),
            VFileType::Device(dev) => dev.ioctl(self, cmd, td),
            VFileType::Blockpool(bp) => bp.ioctl(self, cmd, td),
        }
    }

    pub fn stat(&self, td: Option<&VThread>) -> Result<Stat, Box<dyn Errno>> {
        match &self.ty {
            VFileType::Vnode(vn) => vn.stat(self, td),
            VFileType::Socket(so) | VFileType::IpcSocket(so) => so.stat(self, td),
            VFileType::KernelQueue(kq) => kq.stat(self, td),
            VFileType::SharedMemory(shm) => shm.stat(self, td),
            VFileType::Device(dev) => dev.stat(self, td),
            VFileType::Blockpool(bp) => bp.stat(self, td),
        }
    }

    pub fn truncate(
        &self,
        length: TruncateLength,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        match &self.ty {
            VFileType::Vnode(vn) => vn.truncate(self, length, td),
            VFileType::Socket(so) | VFileType::IpcSocket(so) => so.truncate(self, length, td),
            VFileType::KernelQueue(kq) => kq.truncate(self, length, td),
            VFileType::SharedMemory(shm) => shm.truncate(self, length, td),
            VFileType::Device(dev) => dev.truncate(self, length, td),
            VFileType::Blockpool(bp) => bp.truncate(self, length, td),
        }
    }
}

impl Seek for VFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.seekable_vnode().ok_or(ErrorKind::Other)?;

        // Negative seeks should not be allowed here
        let offset: u64 = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::Current(_) => {
                todo!()
            }
            SeekFrom::End(_) => {
                todo!()
            }
        };

        *self.offset.write() = if let Ok(offset) = offset.try_into() {
            offset
        } else {
            todo!()
        };

        Ok(offset as u64)
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        *self.offset.write() = 0;

        Ok(())
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        if let Ok(offset) = (*self.offset.read()).try_into() {
            Ok(offset)
        } else {
            todo!()
        }
    }
}

impl Read for VFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let total = buf.len();

        let ref mut iovec = IoVec::from_slice(buf);

        let mut offset = self.offset.write();

        let mut uio = UioMut::from_single_vec(iovec, *offset);

        if let Err(e) = VFile::read(self, &mut uio, None) {
            println!("Error: {:?}", e);

            todo!()
        };

        let read = total - uio.bytes_left;

        if let Ok(read) = TryInto::<i64>::try_into(read) {
            *offset += read;
        } else {
            todo!()
        }

        Ok(read)
    }
}

/// Type of [`VFile`].
#[derive(Debug)]
pub enum VFileType {
    Vnode(Arc<Vnode>),               // DTYPE_VNODE = 1
    Socket(Arc<Socket>),             // DTYPE_SOCKET = 2,
    KernelQueue(Arc<KernelQueue>),   // DTYPE_KQUEUE = 5,
    SharedMemory(Arc<SharedMemory>), // DTYPE_SHM = 8,
    Device(Arc<CharacterDevice>),    // DTYPE_DEV = 11,
    IpcSocket(Arc<Socket>),          // DTYPE_IPCSOCKET = 15,
    Blockpool(Arc<BlockPool>),       // DTYPE_BLOCKPOOL = 17,
}

bitflags! {
    /// Flags for [`VFile`].
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct VFileFlags: u32 {
        const READ = 0x00000001; // FREAD
        const WRITE = 0x00000002; // FWRITE
    }
}

/// An implementation of `fileops` structure.
pub trait FileBackend: Debug + Send + Sync + 'static {
    #[allow(unused_variables)]
    /// An implementation of `fo_read`.
    fn read(
        self: &Arc<Self>,
        file: &VFile,
        buf: &mut UioMut,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(DefaultFileBackendError::ReadNotSupported))
    }

    #[allow(unused_variables)]
    /// An implementation of `fo_write`.
    fn write(
        self: &Arc<Self>,
        file: &VFile,
        buf: &mut Uio,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(DefaultFileBackendError::WriteNotSupported))
    }

    #[allow(unused_variables)]
    /// An implementation of `fo_ioctl`.
    fn ioctl(
        self: &Arc<Self>,
        file: &VFile,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(DefaultFileBackendError::IoctlNotSupported))
    }

    #[allow(unused_variables)]
    /// An implementation of `fo_poll`.
    fn poll(self: &Arc<Self>, file: &VFile, events: PollEvents, td: &VThread) -> PollEvents;

    #[allow(unused_variables)]
    /// An implementation of `fo_stat`.
    fn stat(self: &Arc<Self>, file: &VFile, td: Option<&VThread>) -> Result<Stat, Box<dyn Errno>>;

    #[allow(unused_variables)]
    /// An implementation of `fo_truncate`.
    fn truncate(
        self: &Arc<Self>,
        file: &VFile,
        length: TruncateLength,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(DefaultFileBackendError::TruncateNotSupported))
    }
}

#[derive(Debug, Error, Errno)]
pub enum DefaultFileBackendError {
    #[error("reading is not supported")]
    #[errno(ENXIO)]
    ReadNotSupported,

    #[error("writing is not supported")]
    #[errno(ENXIO)]
    WriteNotSupported,

    #[error("ioctl is not supported")]
    #[errno(ENOTTY)]
    IoctlNotSupported,

    #[error("truncating is not supported")]
    #[errno(ENXIO)]
    TruncateNotSupported,

    /// This is used by some file backends to indicate that the operation is not supported.
    #[error("invalid value provided")]
    #[errno(EINVAL)]
    InvalidValue,

    #[error("operation is not supported")]
    #[errno(EOPNOTSUPP)]
    OperationNotSupported,
}
