use super::{IoCmd, IoLen, IoVecMut, Stat, TruncateLength, Vnode};
use crate::errno::{Errno, EINVAL, ENOTTY, ENXIO, EOPNOTSUPP};
use crate::fs::{IoVec, PollEvents};
use crate::process::VThread;
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
    ty: VFileType,             // f_type
    flags: VFileFlags,         // f_flag
    vnode: Option<Arc<Vnode>>, // f_vnode
    offset: Gutex<u64>,        // f_offset
    backend: Box<dyn FileBackend>,
}

impl VFile {
    pub fn new(
        ty: VFileType,
        flags: VFileFlags,
        vnode: Option<Arc<Vnode>>,
        backend: Box<dyn FileBackend>,
    ) -> Self {
        let gg = GutexGroup::new();

        Self {
            ty,
            flags,
            vnode,
            offset: gg.spawn(0),
            backend,
        }
    }

    pub fn ty(&self) -> &VFileType {
        &self.ty
    }

    pub fn flags(&self) -> VFileFlags {
        self.flags
    }

    pub fn is_seekable(&self) -> bool {
        self.backend.is_seekable()
    }

    pub fn vnode(&self) -> Option<&Arc<Vnode>> {
        self.vnode.as_ref()
    }

    pub fn ioctl(&self, cmd: IoCmd, td: Option<&VThread>) -> Result<(), Box<dyn Errno>> {
        self.backend.ioctl(self, cmd, td)
    }

    pub fn stat(&self, td: Option<&VThread>) -> Result<Stat, Box<dyn Errno>> {
        self.backend.stat(self, td)
    }

    pub fn truncate(
        &self,
        length: TruncateLength,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        self.backend.truncate(self, length, td)
    }
}

impl Seek for VFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        use std::io::Error;

        // Check if seekable.
        if !self.backend.is_seekable() {
            return Err(Error::from(ErrorKind::Unsupported));
        }

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

        *self.offset.get_mut() = offset;

        Ok(offset)
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        *self.offset.get_mut() = 0;
        Ok(())
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        Ok(*self.offset.get_mut())
    }
}

impl Read for VFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = IoLen::from_usize(buf.len()).unwrap_or(IoLen::MAX);
        let mut buf = unsafe { IoVecMut::new(buf.as_mut_ptr(), len) };
        let off = *self.offset.get_mut();
        let read = self
            .backend
            .read(self, off, std::slice::from_mut(&mut buf), None)
            .map_err(|e| std::io::Error::new(ErrorKind::Other, e))?;

        *self.offset.get_mut() += read.get() as u64;

        Ok(read.get())
    }
}

/// Type of [`VFile`].
#[repr(i16)]
#[derive(Debug, Clone, Copy)]
pub enum VFileType {
    Vnode = 1,        // DTYPE_VNODE
    Socket = 2,       // DTYPE_SOCKET
    KernelQueue = 5,  // DTYPE_KQUEUE
    SharedMemory = 8, // DTYPE_SHM
    Device = 11,      // DTYPE_DEV
    IpcSocket = 15,   // DTYPE_IPCSOCKET
    Blockpool = 17,   // DTYPE_BLOCKPOOL
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
    /// Implementation of `fo_flags` with `DFLAG_SEEKABLE`.
    fn is_seekable(&self) -> bool;

    /// An implementation of `fo_read`.
    fn read(
        &self,
        file: &VFile,
        off: u64,
        buf: &mut [IoVecMut],
        td: Option<&VThread>,
    ) -> Result<IoLen, Box<dyn Errno>> {
        Err(Box::new(DefaultFileBackendError::ReadNotSupported))
    }

    /// An implementation of `fo_write`.
    fn write(
        &self,
        file: &VFile,
        off: u64,
        buf: &[IoVec],
        td: Option<&VThread>,
    ) -> Result<IoLen, Box<dyn Errno>> {
        Err(Box::new(DefaultFileBackendError::WriteNotSupported))
    }

    /// An implementation of `fo_ioctl`.
    fn ioctl(&self, file: &VFile, cmd: IoCmd, td: Option<&VThread>) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(DefaultFileBackendError::IoctlNotSupported))
    }

    /// An implementation of `fo_poll`.
    fn poll(&self, file: &VFile, events: PollEvents, td: &VThread) -> PollEvents;

    /// An implementation of `fo_stat`.
    fn stat(&self, file: &VFile, td: Option<&VThread>) -> Result<Stat, Box<dyn Errno>>;

    /// An implementation of `fo_truncate`.
    fn truncate(
        &self,
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
