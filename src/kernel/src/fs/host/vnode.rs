use super::file::HostFile;
use super::{get_vnode, GetVnodeError, HostFs};
use crate::errno::{Errno, EEXIST, EIO, ENOENT, ENOTDIR};
use crate::fs::{Access, IoCmd, Mode, OpenFlags, VFile, Vnode, VnodeAttrs, VnodeType};
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
    fn access(
        self: Arc<Self>,
        _: &Arc<Vnode>,
        _: Option<&VThread>,
        _: Access,
    ) -> Result<(), Box<dyn Errno>> {
        // TODO: Check how the PS4 check file permission for exfatfs.
        Ok(())
    }

    fn getattr(self: Arc<Self>, vn: &Arc<Vnode>) -> Result<VnodeAttrs, Box<dyn Errno>> {
        // Get file size.
        let size = self.file.len().map_err(GetAttrError::GetSizeFailed)?;

        // TODO: Check how the PS4 assign file permissions for exfatfs.
        let mode = match vn.ty() {
            VnodeType::Directory(_) => Mode::new(0o555).unwrap(),
            VnodeType::File | VnodeType::Link => todo!(),
            VnodeType::CharacterDevice => unreachable!(), // Character devices should only be in devfs.
        };

        Ok(VnodeAttrs::new(Uid::ROOT, Gid::ROOT, mode, size, u32::MAX))
    }

    fn ioctl(
        self: Arc<Self>,
        #[allow(unused_variables)] vn: &Arc<Vnode>,
        #[allow(unused_variables)] cmd: IoCmd,
        #[allow(unused_variables)] td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }

    fn lookup(
        self: Arc<Self>,
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

                // Lookup the file.
                Cow::Owned(self.file.open(name).map_err(LookupError::OpenFailed)?)
            }
        };

        // Get vnode.
        let vn = get_vnode(&self.fs, vn.fs(), &file).map_err(LookupError::GetVnodeFailed)?;

        Ok(vn)
    }

    fn mkdir(
        self: Arc<Self>,
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

        Ok(Vnode::new(
            parent.fs(),
            VnodeType::Directory(false),
            "exfatfs",
            VnodeBackend::new(self.fs.clone(), dir),
        ))
    }

    #[allow(unused_variables)] // TODO: remove when implementing.
    fn open(
        self: Arc<Self>,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        mode: OpenFlags,
        file: Option<&mut VFile>,
    ) -> Result<(), Box<dyn Errno>> {
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

#[derive(Debug, Error, Errno)]
enum MkDirError {
    #[error("couldn't create directory")]
    #[errno(EIO)]
    CreateFailed(#[source] std::io::Error),

    #[error("directory already exists")]
    #[errno(EEXIST)]
    AlreadyExists,
}

impl From<std::io::Error> for MkDirError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::AlreadyExists => MkDirError::AlreadyExists,
            _ => MkDirError::CreateFailed(e),
        }
    }
}
