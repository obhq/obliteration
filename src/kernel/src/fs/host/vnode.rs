use super::file::HostFile;
use super::{get_vnode, GetVnodeError, HostFs};
use crate::errno::{Errno, EIO, ENOENT, ENOTDIR};
use crate::fs::{Access, Mode, OpenFlags, VFile, Vnode, VnodeAttrs, VnodeType};
use crate::process::VThread;
use crate::ucred::{Gid, Uid};
use macros::Errno;
use std::borrow::Cow;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

/// An implementation of [`crate::fs::VnodeBackend`].
#[derive(Debug)]
pub struct VnodeBackend {
    fs: Arc<HostFs>,
    file: HostFile,
}

impl VnodeBackend {
    pub fn new(fs: Arc<HostFs>, file: HostFile) -> Self {
        Self { fs, file }
    }
}

impl crate::fs::VnodeBackend for VnodeBackend {
    fn access(
        self: Arc<Self>,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        mode: Access,
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
            VnodeType::Character => unreachable!(), // Character devices should only be in devfs.
        };

        Ok(VnodeAttrs::new(Uid::ROOT, Gid::ROOT, mode, size, u32::MAX))
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
        if name == "." {
            return Ok(vn.clone());
        }

        let path = match name {
            ".." => Cow::Borrowed(self.file.path().parent().unwrap()),
            _ => {
                if name.contains(|c| c == '/' || c == '\\') {
                    return Err(Box::new(LookupError::InvalidName));
                }

                Cow::Owned(self.file.path().join(name))
            }
        };

        // Get vnode.
        let vn = get_vnode(&self.fs, vn.fs(), Some(&path)).map_err(LookupError::GetVnodeFailed)?;

        Ok(vn)
    }

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
#[derive(Debug, Error)]
enum LookupError {
    #[error("current file is not a directory")]
    NotDirectory,

    #[error("cannot resolve '..' on the root directory")]
    DotdotOnRoot,

    #[error("access denied")]
    AccessDenied(#[source] Box<dyn Errno>),

    #[error("name contains unsupported characters")]
    InvalidName,

    #[error("cannot get vnode")]
    GetVnodeFailed(#[source] GetVnodeError),
}

impl Errno for LookupError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::NotDirectory => ENOTDIR,
            Self::DotdotOnRoot | Self::GetVnodeFailed(_) => EIO,
            Self::AccessDenied(e) => e.errno(),
            Self::InvalidName => ENOENT,
        }
    }
}
