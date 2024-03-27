use super::file::HostFile;
use super::{GetVnodeError, HostFs};
use crate::errno::{Errno, EEXIST, EIO, ENOENT, ENOTDIR};
use crate::fs::{
    Access, IoCmd, Mode, OpenFlags, Uio, UioMut, VFileType, Vnode, VnodeAttrs, VnodeType,
};
use crate::process::VThread;
use crate::ucred::{Gid, Uid};
use macros::Errno;
use std::borrow::Cow;
use std::sync::Arc;
use thiserror::Error;

/// An implementation of [`crate::fs::VnodeBackend`].
#[derive(Debug)]
pub struct VnodeBackend {
    fs: Arc<HostFs>,
    file: Arc<HostFile>,
}

impl VnodeBackend {
    pub fn new(fs: Arc<HostFs>, file: Arc<HostFile>) -> Self {
        Self { fs, file }
    }
}

impl crate::fs::VnodeBackend for VnodeBackend {
    fn access(&self, _: &Arc<Vnode>, _: Option<&VThread>, _: Access) -> Result<(), Box<dyn Errno>> {
        // TODO: Check how the PS4 check file permission for exfatfs.
        Ok(())
    }

    fn getattr(&self, vn: &Arc<Vnode>) -> Result<VnodeAttrs, Box<dyn Errno>> {
        // Get file size.
        let size = self.file.len().map_err(GetAttrError::GetSizeFailed)?;

        // TODO: Check how the PS4 assign file permissions for exfatfs.
        let mode = match vn.ty() {
            VnodeType::Directory(_) => Mode::new(0o555).unwrap(),
            VnodeType::File | VnodeType::Link => todo!(),
            VnodeType::CharacterDevice => unreachable!(), // Character devices should only be in devfs.
        };

        Ok(VnodeAttrs {
            uid: Uid::ROOT,
            gid: Gid::ROOT,
            mode,
            size,
            fsid: u32::MAX,
        })
    }

    fn ioctl(
        &self,
        #[allow(unused_variables)] vn: &Arc<Vnode>,
        #[allow(unused_variables)] cmd: IoCmd,
        #[allow(unused_variables)] td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }

    fn lookup(
        &self,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        name: &str,
    ) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        // Check if directory.
        match vn.ty() {
            VnodeType::Directory(root) => {
                if name == ".." && *root {
                    return Err(Box::new(LookupError::DotdotOnRoot));
                }
            }
            _ => return Err(Box::new(LookupError::NotDirectory)),
        }

        // Check if directory is accessible.
        vn.access(td, Access::EXEC)
            .map_err(LookupError::AccessDenied)?;

        // Check name.
        let file = match name {
            "." => return Ok(vn.clone()),
            ".." => Cow::Borrowed(self.file.parent().unwrap()),
            _ => {
                // Don't allow name to be a file path.
                if name.contains(|c| c == '/' || c == '\\') {
                    return Err(Box::new(LookupError::InvalidName));
                }

                let host_file = self.file.open(name).map_err(LookupError::OpenFailed)?;

                // Lookup the file.
                Cow::Owned(host_file)
            }
        };

        // Get vnode.
        let vn = self
            .fs
            .get_vnode(vn.mount(), &file)
            .map_err(LookupError::GetVnodeFailed)?;

        Ok(vn)
    }

    fn mkdir(
        &self,
        parent: &Arc<Vnode>,
        name: &str,
        mode: u32,
        td: Option<&VThread>,
    ) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        parent.access(td, Access::WRITE)?;

        let dir = self
            .file
            .mkdir(name, mode)
            .map_err(|e| MkDirError::from(e))?;

        let vn = self.fs.get_vnode(parent.mount(), &dir)?;

        Ok(vn)
    }

    #[allow(unused_variables)] // TODO: remove when implementing.
    fn open(
        &self,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        flags: OpenFlags,
    ) -> Result<VFileType, Box<dyn Errno>> {
        // TODO: implement what the PS4 does here

        Ok(VFileType::Vnode(vn.clone()))
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn read(
        &self,
        vn: &Arc<Vnode>,
        buf: &mut UioMut,
        offset: i64,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        let read = self.file.read(buf, offset).map_err(ReadError::ReadFailed)?;

        Ok(read)
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn write(
        &self,
        vn: &Arc<Vnode>,
        buf: &mut Uio,
        offset: i64,
        td: Option<&VThread>,
    ) -> Result<usize, Box<dyn Errno>> {
        todo!()
    }
}

/// Represents an error when [`getattr()`] fails.
#[derive(Debug, Error, Errno)]
enum GetAttrError {
    #[error("cannot get file size")]
    #[errno(EIO)]
    GetSizeFailed(#[source] std::io::Error),
}

/// Represents an error when [`lookup()`] fails.
#[derive(Debug, Error, Errno)]
enum LookupError {
    #[error("current file is not a directory")]
    #[errno(ENOTDIR)]
    NotDirectory,

    #[error("cannot resolve '..' on the root directory")]
    #[errno(EIO)]
    DotdotOnRoot,

    #[error("access denied")]
    AccessDenied(#[source] Box<dyn Errno>),

    #[error("name contains unsupported characters")]
    #[errno(ENOENT)]
    InvalidName,

    #[error("couldn't open the specified file")]
    #[errno(EIO)]
    OpenFailed(#[source] std::io::Error),

    #[error("cannot get vnode")]
    GetVnodeFailed(#[source] GetVnodeError),
}

/// Represents an error when [`VnodeBackend::mkdir()`] fails.
#[derive(Debug, Error, Errno)]
enum MkDirError {
    #[error("couldn't create directory")]
    #[errno(EIO)]
    CreateFailed(#[source] std::io::Error),

    #[error("directory already exists")]
    #[errno(EEXIST)]
    AlreadyExists,

    #[error("couldn't get vnode")]
    GetVnodeFailed(#[from] GetVnodeError),
}

impl From<std::io::Error> for MkDirError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::AlreadyExists => MkDirError::AlreadyExists,
            _ => MkDirError::CreateFailed(e),
        }
    }
}

#[derive(Debug, Error, Errno)]
enum ReadError {
    #[error("read failed")]
    #[errno(EIO)]
    ReadFailed(#[from] std::io::Error),
}
