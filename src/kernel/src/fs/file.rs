use super::socket::Socket;
use super::{IoCmd, Vnode};
use crate::errno::Errno;
use crate::process::VThread;
use bitflags::bitflags;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;

/// An implementation of `file` structure.
#[derive(Debug)]
pub struct VFile {
    ty: VFileType,          // f_type + f_data
    ops: &'static VFileOps, // f_ops
    flags: VFileFlags,      // f_flag
}

impl VFile {
    pub(super) fn new(ty: VFileType, ops: &'static VFileOps) -> Self {
        Self {
            ty,
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

    pub fn data_as_socket(&self) -> Option<&Socket> {
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
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        todo!()
    }
}

impl Read for VFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        todo!()
    }
}

/// Type of [`VFile`].
#[derive(Debug)]
pub enum VFileType {
    Vnode(Arc<Vnode>),    // DTYPE_VNODE
    Socket(Arc<Socket>),  // DTYPE_SOCKET
    Socket2(Arc<Socket>), // TODO: figure out what this is
}

/// An implementation of `fileops` structure.
#[derive(Debug)]
pub struct VFileOps {
    pub write: fn(&VFile, &[u8], Option<&VThread>) -> Result<usize, Box<dyn Errno>>,
    pub ioctl: fn(&VFile, IoCmd, &mut [u8], Option<&VThread>) -> Result<(), Box<dyn Errno>>,
}

bitflags! {
    /// Flags for [`VFile`].
    #[derive(Debug, Clone, Copy)]
    pub struct VFileFlags: u32 {
        const FREAD = 0x00000001;
        const FWRITE = 0x00000002;
    }
}

pub const FILE_GROUP: u8 = b'f';

pub const FIOCLEX: IoCmd = IoCmd::io(FILE_GROUP, 1);
pub const FIONCLEX: IoCmd = IoCmd::io(FILE_GROUP, 2);
pub const FIONREAD: IoCmd = IoCmd::ior::<i32>(FILE_GROUP, 0x7f);
pub const FIONBIO: IoCmd = IoCmd::iow::<i32>(FILE_GROUP, 0x7e);
pub const FIOASYNC: IoCmd = IoCmd::iow::<i32>(FILE_GROUP, 0x7d);
pub const FIOSETOWN: IoCmd = IoCmd::iow::<i32>(FILE_GROUP, 0x7c);
pub const FIOGETOWN: IoCmd = IoCmd::ior::<i32>(FILE_GROUP, 0x7b);
pub const FIODTYPE: IoCmd = IoCmd::ior::<i32>(FILE_GROUP, 0x7a);
pub const FIOGETLBA: IoCmd = IoCmd::ior::<i32>(FILE_GROUP, 0x79);

struct FioDgNameArg {
    len: i32,
    buf: *mut (),
}

pub const FIODGNAME: IoCmd = IoCmd::ior::<FioDgNameArg>(FILE_GROUP, 0x78);
pub const FIONWRITE: IoCmd = IoCmd::ior::<i32>(FILE_GROUP, 0x77);
pub const FIONSPACE: IoCmd = IoCmd::ior::<i32>(FILE_GROUP, 0x76);

pub const FIOSEEKDATA: IoCmd = IoCmd::ior::<usize>(FILE_GROUP, 0x61);
pub const FIOSEEKHOLE: IoCmd = IoCmd::ior::<usize>(FILE_GROUP, 0x62);
