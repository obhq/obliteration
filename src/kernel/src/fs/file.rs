use super::{IoCmd, Vnode};
use crate::errno::Errno;
use crate::net::Socket;
use crate::process::VThread;
use bitflags::bitflags;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::Arc;

/// An implementation of `file` structure.
#[derive(Debug)]
pub struct VFile {
    ty: VFileType,          // f_type + f_data
    ops: &'static VFileOps, // f_ops
    flags: VFileFlags,      // f_flag
    offset: u64,            // f_offset
}

impl VFile {
    pub(super) fn new(ty: VFileType, ops: &'static VFileOps, flags: VFileFlags) -> Self {
        Self {
            ty,
            ops,
            flags,
            offset: 0,
        }
    }

    pub fn flags(&self) -> VFileFlags {
        self.flags
    }

    pub fn flags_mut(&mut self) -> &mut VFileFlags {
        &mut self.flags
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn data_as_socket(&self) -> Option<&Arc<Socket>> {
        match &self.ty {
            VFileType::Socket(so) | VFileType::Socket2(so) => Some(so),
            _ => None,
        }
    }

    pub fn data_as_vnode(&self) -> Option<&Arc<Vnode>> {
        match &self.ty {
            VFileType::Vnode(v) => Some(v),
            _ => None,
        }
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
    Vnode(Arc<Vnode>),    // DTYPE_VNODE
    Socket(Arc<Socket>),  // DTYPE_SOCKET
    Socket2(Arc<Socket>), // TODO: figure out what exactly this is
}

/// An implementation of `fileops` structure.
#[derive(Debug)]
pub struct VFileOps {
    pub read: VFileRead,
    pub write: VFileWrite,
    pub ioctl: VFileIoctl,
}

type VFileRead = fn(&VFile, &mut [u8], Option<&VThread>) -> Result<usize, Box<dyn Errno>>;
type VFileWrite = fn(&VFile, &[u8], Option<&VThread>) -> Result<usize, Box<dyn Errno>>;
type VFileIoctl = fn(&VFile, IoCmd, &mut [u8], Option<&VThread>) -> Result<(), Box<dyn Errno>>;

bitflags! {
    /// Flags for [`VFile`].
    #[derive(Debug, Clone, Copy)]
    pub struct VFileFlags: u32 {
        const FREAD = 0x00000001;
        const FWRITE = 0x00000002;
    }
}
