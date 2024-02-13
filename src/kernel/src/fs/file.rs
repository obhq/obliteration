use super::{IoCmd, Vnode};
use crate::dmem::BlockPool;
use crate::errno::Errno;
use crate::errno::{ENOTTY, ENXIO};
use crate::kqueue::KernelQueue;
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

    pub fn read(&self, data: &mut [u8], td: Option<&VThread>) -> Result<usize, Box<dyn Errno>> {
        match self.backend {
            VFileType::Vnode(ref vn) => vn.read(self, data, td),
            VFileType::KernelQueue(ref kq) => kq.read(self, data, td),
            VFileType::Blockpool(ref bp) => bp.read(self, data, td),
        }
    }

    pub fn write(&self, data: &[u8], td: Option<&VThread>) -> Result<usize, Box<dyn Errno>> {
        match self.backend {
            VFileType::Vnode(ref vn) => vn.write(self, data, td),
            VFileType::KernelQueue(ref kq) => kq.write(self, data, td),
            VFileType::Blockpool(ref bp) => bp.write(self, data, td),
        }
    }

    pub fn ioctl(
        &self,
        cmd: IoCmd,
        data: &mut [u8],
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        match self.backend {
            VFileType::Vnode(ref vn) => vn.ioctl(self, cmd, data, td),
            VFileType::KernelQueue(ref kq) => kq.ioctl(self, cmd, data, td),
            VFileType::Blockpool(ref bp) => bp.ioctl(self, cmd, data, td),
        }
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
pub enum VFileType {
    Vnode(Arc<Vnode>),             // DTYPE_VNODE = 1
    KernelQueue(Arc<KernelQueue>), // DTYPE_KQUEUE = 5,
    Blockpool(Arc<BlockPool>),     // DTYPE_BPOOL = 17,
}

bitflags! {
    /// Flags for [`VFile`].
    #[derive(Debug, Clone, Copy)]
    pub struct VFileFlags: u32 {
        const FREAD = 0x00000001;
        const FWRITE = 0x00000002;
    }
}

/// An implementation of `fileops` structure.
pub trait FileBackend: Debug + Send + Sync + 'static {
    #[allow(unused_variables)]
    fn read(
        self: &Arc<Self>,
        file: &VFile,
        buf: &mut [u8],
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        Err(Box::new(DefaultError::ReadNotSupported))
    }

    #[allow(unused_variables)]
    fn write(
        self: &Arc<Self>,
        file: &VFile,
        buf: &[u8],
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        Err(Box::new(DefaultError::WriteNotSupported))
    }

    #[allow(unused_variables)]
    fn ioctl(
        self: &Arc<Self>,
        file: &VFile,
        cmd: IoCmd,
        data: &mut [u8],
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(DefaultError::IoctlNotSupported))
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

    #[error("iocll is not supported")]
    #[errno(ENOTTY)]
    IoctlNotSupported,
}
