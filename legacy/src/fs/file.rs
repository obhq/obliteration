use super::{IoCmd, IoLen, IoVecMut, Stat, TruncateLength, Vnode};
use crate::errno::{Errno, EINVAL, ENOTTY, ENXIO, EOPNOTSUPP};
use crate::fs::{IoVec, PollEvents};
use crate::process::VThread;
use bitflags::bitflags;
use gmtx::{Gutex, GutexGroup};
use macros::Errno;
use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::sync::Arc;
use thiserror::Error;

/// An implementation of `file` structure.
#[derive(Debug)]
pub struct VFile {
    flags: VFileFlags,  // f_flag
    offset: Gutex<u64>, // f_offset
    backend: Box<dyn FileBackend>,
}

impl VFile {
    pub fn new(flags: VFileFlags, backend: Box<dyn FileBackend>) -> Self {
        let gg = GutexGroup::new();

        Self {
            flags,
            offset: gg.spawn(0),
            backend,
        }
    }

    pub fn flags(&self) -> VFileFlags {
        self.flags
    }

    pub fn is_seekable(&self) -> bool {
        self.backend.is_seekable()
    }

    pub fn vnode(&self) -> Option<&Arc<Vnode>> {
        self.backend.vnode()
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

    /// Gets the `f_data` associated with this file.
    ///
    /// File implementation should use this method to check if the file has expected type. This
    /// method should be impossible for non-file implementation to call because it required an
    /// implementation of [`FileBackend`], which should not exposed by the subsystem itself. This
    /// also imply the other subsystems cannot call this method with the other subsystem
    /// implementation.
    pub fn backend<T: FileBackend>(&self) -> Option<&T> {
        // TODO: Use Any::downcast_ref() when https://github.com/rust-lang/rust/issues/65991 is
        // stabilized. Our current implementation here is copied from Any::downcast_ref().
        let b = self.backend.as_ref();

        if b.type_id() == TypeId::of::<T>() {
            Some(unsafe { &*(b as *const dyn FileBackend as *const T) })
        } else {
            None
        }
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

bitflags! {
    /// Flags for [`VFile`].
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct VFileFlags: u32 {
        const READ = 0x00000001; // FREAD
        const WRITE = 0x00000002; // FWRITE
    }
}

/// An implementation of `fileops` structure.
///
/// The implementation is internal to the subsystem itself so it should not expose itself to the
/// outside.
pub trait FileBackend: Any + Debug + Send + Sync + 'static {
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

    /// Get a vnode associated with this file (if any).
    ///
    /// Usually this will be [`Some`] if the file was opened from a filesystem (e.g. `/dev/null`)
    /// and [`None`] if the file is not living on the filesystem (e.g. kqueue).
    fn vnode(&self) -> Option<&Arc<Vnode>>;
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
