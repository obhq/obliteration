use super::file::HostFile;
use super::{get_vnode, GetVnodeError};
use crate::errno::{Errno, EIO, ENOENT, ENOTDIR};
use crate::fs::{Access, OpenFlags, VFile, Vnode, VnodeType, VopVector, DEFAULT_VNODEOPS};
use crate::process::VThread;
use std::borrow::Cow;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

pub static VNODE_OPS: VopVector = VopVector {
    default: Some(&DEFAULT_VNODEOPS),
    access: Some(access),
    accessx: None,
    lookup: Some(lookup),
    open: Some(open),
};

fn access(_: &Arc<Vnode>, _: Option<&VThread>, _: Access) -> Result<(), Box<dyn Errno>> {
    // TODO: Check how the PS4 check file permission for exfatfs.
    Ok(())
}

fn lookup(vn: &Arc<Vnode>, td: Option<&VThread>, name: &str) -> Result<Arc<Vnode>, Box<dyn Errno>> {
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
    if let Err(e) = vn.access(td, Access::EXEC) {
        return Err(Box::new(LookupError::AccessDenied(e)));
    }

    // Check name.
    if name == "." {
        return Ok(vn.clone());
    }

    let host = vn.data().downcast_ref::<HostFile>().unwrap();
    let path = match name {
        ".." => Cow::Borrowed(host.path().parent().unwrap()),
        _ => {
            if name.contains(|c| c == '/' || c == '\\') {
                return Err(Box::new(LookupError::InvalidName));
            }

            Cow::Owned(host.path().join(name))
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
    _: &Arc<Vnode>,
    _: Option<&VThread>,
    _: OpenFlags,
    _: Option<&mut VFile>,
) -> Result<(), Box<dyn Errno>> {
    todo!()
}

/// Represents an error when [`lookup()`] is failed.
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
