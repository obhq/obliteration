use super::{IoCmd, Offset, Stat, TruncateLength, Uio, UioMut, Vnode};
use crate::dmem::BlockPool;
use crate::errno::Errno;
use crate::errno::{EINVAL, ENOTTY, ENXIO};
use crate::kqueue::KernelQueue;
use crate::net::Socket;
use crate::process::VThread;
use bitflags::bitflags;
use macros::Errno;
use std::fmt::Debug;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::Arc;
use thiserror::Error;

/// An implementation of `file` structure.
#[derive(Debug)]
pub struct VFile {
    backend: VFileType, // f_type
    flags: VFileFlags,  // f_flag
}

impl VFile {
    pub(super) fn new(backend: VFileType) -> Self {
        Self {
            backend,
            flags: VFileFlags::empty(),
        }
    }

    pub fn flags(&self) -> VFileFlags {
        self.flags
    }

    pub fn flags_mut(&mut self) -> &mut VFileFlags {
        &mut self.flags
    }

    pub fn vnode(&self) -> Option<Arc<Vnode>> {
        todo!()
    }

    /// See `dofileread` on the PS4 for a reference.
    pub fn do_read(
        &self,
        mut uio: UioMut,
        off: Offset,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        if uio.bytes_left == 0 {
            return Ok(0);
        }

        // TODO: consider implementing ktrace.

        let res = self.read(&mut uio, td);

        if let Err(ref e) = res {
            todo!()
        }

        res
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

        let res = self.write(&mut uio, td);

        if let Err(ref e) = res {
            todo!()
        }

        res
    }

    fn read(&self, buf: &mut UioMut, td: Option<&VThread>) -> Result<usize, Box<dyn Errno>> {
        match &self.backend {
            VFileType::Vnode(vn) => vn.read(self, buf, td),
            VFileType::Socket(so) | VFileType::IpcSocket(so) => so.read(self, buf, td),
            VFileType::KernelQueue(kq) => kq.read(self, buf, td),
            VFileType::Blockpool(bp) => bp.read(self, buf, td),
        }
    }

    fn write(&self, buf: &mut Uio, td: Option<&VThread>) -> Result<usize, Box<dyn Errno>> {
        match &self.backend {
            VFileType::Vnode(vn) => vn.write(self, buf, td),
            VFileType::Socket(so) | VFileType::IpcSocket(so) => so.write(self, buf, td),
            VFileType::KernelQueue(kq) => kq.write(self, buf, td),
            VFileType::Blockpool(bp) => bp.write(self, buf, td),
        }
    }

    /// See `fo_ioctl` on the PS4 for a reference.
    pub fn ioctl(&self, cmd: IoCmd, td: Option<&VThread>) -> Result<(), Box<dyn Errno>> {
        match &self.backend {
            VFileType::Vnode(vn) => vn.ioctl(self, cmd, td),
            VFileType::Socket(so) | VFileType::IpcSocket(so) => so.ioctl(self, cmd, td),
            VFileType::KernelQueue(kq) => kq.ioctl(self, cmd, td),
            VFileType::Blockpool(bp) => bp.ioctl(self, cmd, td),
        }
    }

    pub fn stat(&self, td: Option<&VThread>) -> Result<Stat, Box<dyn Errno>> {
        match &self.backend {
            VFileType::Vnode(vn) => vn.stat(self, td),
            VFileType::Socket(so) | VFileType::IpcSocket(so) => so.stat(self, td),
            VFileType::KernelQueue(kq) => kq.stat(self, td),
            VFileType::Blockpool(bp) => bp.stat(self, td),
        }
    }

    pub fn truncate(
        &self,
        length: TruncateLength,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        match &self.backend {
            VFileType::Vnode(vn) => vn.truncate(self, length, td),
            VFileType::Socket(so) | VFileType::IpcSocket(so) => so.truncate(self, length, td),
            VFileType::KernelQueue(kq) => kq.truncate(self, length, td),
            VFileType::Blockpool(bp) => bp.truncate(self, length, td),
        }
    }

    pub fn is_seekable(&self) -> bool {
        matches!(self.backend, VFileType::Vnode(_))
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
pub enum VFileType {
    Vnode(Arc<Vnode>),             // DTYPE_VNODE = 1
    Socket(Arc<Socket>),           // DTYPE_SOCKET = 2,
    KernelQueue(Arc<KernelQueue>), // DTYPE_KQUEUE = 5,
    IpcSocket(Arc<Socket>),        // DTYPE_IPCSOCKET = 15,
    Blockpool(Arc<BlockPool>),     // DTYPE_BLOCKPOOL = 17,
}

bitflags! {
    /// Flags for [`VFile`].
    #[derive(Debug, Clone, Copy)]
    pub struct VFileFlags: u32 {
        const READ = 0x00000001; // FREAD
        const WRITE = 0x00000002; // FWRITE
    }
}

/// An implementation of `fileops` structure.
pub trait FileBackend: Debug + Send + Sync + 'static {
    #[allow(unused_variables)]
    fn read(
        self: &Arc<Self>,
        file: &VFile,
        buf: &mut UioMut,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        Err(Box::new(DefaultError::ReadNotSupported))
    }

    #[allow(unused_variables)]
    fn write(
        self: &Arc<Self>,
        file: &VFile,
        buf: &mut Uio,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        Err(Box::new(DefaultError::WriteNotSupported))
    }

    #[allow(unused_variables)]
    fn ioctl(
        self: &Arc<Self>,
        file: &VFile,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(DefaultError::IoctlNotSupported))
    }

    #[allow(unused_variables)]
    fn stat(self: &Arc<Self>, file: &VFile, td: Option<&VThread>) -> Result<Stat, Box<dyn Errno>>;

    #[allow(unused_variables)]
    fn truncate(
        self: &Arc<Self>,
        file: &VFile,
        length: TruncateLength,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(DefaultError::TruncateNotSupported))
    }
}

#[derive(Debug, Error, Errno)]
pub enum DefaultError {
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
}
