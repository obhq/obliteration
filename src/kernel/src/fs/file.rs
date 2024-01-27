use super::socket::Socket;
use super::{IoCmd, Vnode};
use crate::errno::{Errno, EINVAL, ENOTTY};
use crate::fs::VnodeType;
use crate::process::VThread;
use crate::ucred::Privilege;
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use std::io::{Read, Seek, SeekFrom, Write};
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

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
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
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

pub static VNOPS: VFileOps = VFileOps {
    read: vn_read,
    write: vn_write,
    ioctl: vn_ioctl,
};

fn vn_read(file: &VFile, buf: &mut [u8], td: Option<&VThread>) -> Result<usize, Box<dyn Errno>> {
    todo!()
}

fn vn_write(file: &VFile, buf: &[u8], td: Option<&VThread>) -> Result<usize, Box<dyn Errno>> {
    todo!()
}

fn vn_ioctl(
    file: &VFile,
    cmd: IoCmd,
    buf: &mut [u8],
    td: Option<&VThread>,
) -> Result<(), Box<dyn Errno>> {
    let vn = file.data_as_vnode().unwrap();

    match vn.ty() {
        VnodeType::File | VnodeType::Directory(_) => match cmd {
            FIONREAD => {
                let attr = vn.getattr()?;

                let len: &mut i32 = bytemuck::from_bytes_mut(buf);

                *len = (attr.size() - file.offset()).try_into().unwrap();
            }
            FIOCHECKANDMODIFY => {
                td.unwrap().priv_check(Privilege::SCE683)?;

                let _arg: &FioCheckAndModifyArg = bytemuck::from_bytes(buf);

                todo!()
            }
            FIONBIO | FIOASYNC => {}
            _ => vn.ioctl(cmd, buf, td)?,
        },
        _ => return Err(IoctlError::WrongFileType.into()),
    }

    Ok(())
}

pub const FILE_GROUP: u8 = b'f';

pub const FIOCLEX: IoCmd = IoCmd::io(FILE_GROUP, 1);
pub const FIONCLEX: IoCmd = IoCmd::io(FILE_GROUP, 2);
pub const FIONREAD: IoCmd = IoCmd::ior::<i32>(FILE_GROUP, 127);
pub const FIONBIO: IoCmd = IoCmd::iow::<i32>(FILE_GROUP, 126);
pub const FIOASYNC: IoCmd = IoCmd::iow::<i32>(FILE_GROUP, 125);
pub const FIOSETOWN: IoCmd = IoCmd::iow::<i32>(FILE_GROUP, 124);
pub const FIOGETOWN: IoCmd = IoCmd::ior::<i32>(FILE_GROUP, 123);
pub const FIODTYPE: IoCmd = IoCmd::ior::<i32>(FILE_GROUP, 122);
pub const FIOGETLBA: IoCmd = IoCmd::ior::<i32>(FILE_GROUP, 121);

#[repr(C)]
struct FioDgNameArg {
    len: i32,
    buf: *mut u8,
}

pub const FIODGNAME: IoCmd = IoCmd::ior::<FioDgNameArg>(FILE_GROUP, 120);
pub const FIONWRITE: IoCmd = IoCmd::ior::<i32>(FILE_GROUP, 119);
pub const FIONSPACE: IoCmd = IoCmd::ior::<i32>(FILE_GROUP, 118);

pub const FIOSEEKDATA: IoCmd = IoCmd::ior::<usize>(FILE_GROUP, 97);
pub const FIOSEEKHOLE: IoCmd = IoCmd::ior::<usize>(FILE_GROUP, 98);

#[repr(C)]
#[derive(Clone, Copy, Zeroable)]
struct FioCheckAndModifyArg {
    flag: i32,
    _padding: i32,
    unk2: usize,
    unk3: usize,
    path: *const u8,
    unk5: usize,
}

// This should be fine for our usecase.
unsafe impl Pod for FioCheckAndModifyArg {}

/// PS4-specific
pub const FIOCHECKANDMODIFY: IoCmd = IoCmd::iow::<FioCheckAndModifyArg>(FILE_GROUP, 189);

#[derive(Debug, Error)]
pub enum IoctlError {
    #[error("wrong file type")]
    WrongFileType,

    #[error("invalid flag for FIOCHECKANDMODIFY ({0:#x})")]
    InvalidFlag(i32),
}

impl Errno for IoctlError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            IoctlError::WrongFileType => ENOTTY,
            IoctlError::InvalidFlag(_) => EINVAL,
        }
    }
}
