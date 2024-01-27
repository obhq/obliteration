use super::file::HostFile;
use super::{get_vnode, GetVnodeError};
use crate::errno::{Errno, EIO, ENOENT, ENOTDIR};
use crate::fs::{
    Access, LookupOp, Mode, OpenFlags, VFile, VPathComponent, Vnode, VnodeAttrs, VnodeType,
};
use crate::process::VThread;
use crate::ucred::{Gid, Uid};
use std::borrow::Cow;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

/// An implementation of [`crate::fs::VnodeBackend`].
#[derive(Debug)]
pub struct VnodeBackend {
    file: HostFile,
}

impl VnodeBackend {
    pub fn new(file: HostFile) -> Self {
        Self { file }
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
        let size = match self.file.len() {
            Ok(v) => v,
            Err(e) => return Err(Box::new(GetAttrError::GetSizeFailed(e))),
        };

        // TODO: Check how the PS4 assign file permissions for exfatfs.
        let mode = match vn.ty() {
            VnodeType::Directory(_) => Mode::new(0o555).unwrap(),
            VnodeType::Character => unreachable!(), // The character device should only be in the devfs.
        };

        Ok(VnodeAttrs::new(Uid::ROOT, Gid::ROOT, mode, size))
    }

    fn lookup(
        self: Arc<Self>,
        vn: &Arc<Vnode>,
        cn: VPathComponent,
        op: LookupOp,
        td: Option<&VThread>,
    ) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        // Check if directory.
        match vn.ty() {
            VnodeType::Directory(root) => {
                if cn == VPathComponent::DotDot && *root {
                    return Err(Box::new(LookupError::DotDotOnRoot));
                }
            }
            _ => return Err(Box::new(LookupError::NotDirectory)),
        }

        // Check if directory is accessible.
        if let Err(e) = vn.access(td, Access::EXEC) {
            return Err(Box::new(LookupError::AccessDenied(e)));
        }

        // Check name.
        if cn == VPathComponent::Dot {
            return Ok(vn.clone());
        }

        let path = match cn {
            VPathComponent::DotDot => Cow::Borrowed(self.file.path().parent().unwrap()),
            _ => {
                if cn.is_normal_and_contains(|c| c == '/' || c == '\\') {
                    return Err(Box::new(LookupError::InvalidName));
                }

                Cow::Owned(self.file.path().join(cn))
            }
        };

        // Get vnode.
        let vn = match get_vnode(vn.fs(), Some(&path)) {
            Ok(v) => v,
            Err(e) => return Err(Box::new(LookupError::GetVnodeFailed(e))),
        };

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

/// Represents an error when [`getattr()`] was failed.
#[derive(Debug, Error)]
enum GetAttrError {
    #[error("cannot get file size")]
    GetSizeFailed(#[source] std::io::Error),
}

impl Errno for GetAttrError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::GetSizeFailed(_) => EIO,
        }
    }
}

/// Represents an error when [`lookup()`] was failed.
#[derive(Debug, Error)]
enum LookupError {
    #[error("current file is not a directory")]
    NotDirectory,

    #[error("cannot resolve '..' on the root directory")]
    DotDotOnRoot,

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
            Self::DotDotOnRoot | Self::GetVnodeFailed(_) => EIO,
            Self::AccessDenied(e) => e.errno(),
            Self::InvalidName => ENOENT,
        }
    }
}
