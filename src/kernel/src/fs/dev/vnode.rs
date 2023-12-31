use super::dirent::Dirent;
use super::{alloc_vnode, AllocVnodeError};
use crate::errno::{Errno, EIO, ENOENT, ENOTDIR};
use crate::fs::{check_access, DevFs, Vnode, VnodeType, VopVector, DEFAULT_VNODEOPS};
use crate::process::VThread;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

pub static VNODE_OPS: VopVector = VopVector {
    default: Some(&DEFAULT_VNODEOPS),
    access: Some(access),
    accessx: None,
    lookup: Some(lookup),
};

pub static CHARACTER_OPS: VopVector = VopVector {
    default: Some(&DEFAULT_VNODEOPS),
    access: Some(access),
    accessx: None,
    lookup: None,
};

fn access(vn: &Arc<Vnode>, td: Option<&VThread>, access: u32) -> Result<(), Box<dyn Errno>> {
    // Get dirent.
    let mut dirent = vn.data().clone().downcast::<Dirent>().unwrap();
    let is_dir = match vn.ty() {
        VnodeType::Directory(_) => {
            if let Some(v) = dirent.dir() {
                // Is it possible the parent will be gone here?
                dirent = v.upgrade().unwrap();
            }

            true
        }
        _ => false,
    };

    // Get credential.
    let cred = match td {
        Some(v) => v.cred(),
        None => return Ok(()),
    };

    // Get file permissions as atomic.
    let (uid, gid, mode) = {
        let uid = dirent.uid();
        let gid = dirent.gid();
        let mode = dirent.mode();

        (*uid, *gid, *mode)
    };

    // Check access.
    let err = match check_access(cred, uid, gid, mode.into(), access, is_dir) {
        Ok(_) => return Ok(()),
        Err(e) => e,
    };

    // TODO: Check if file is a controlling terminal.
    return Err(Box::new(err));
}

fn lookup(vn: &Arc<Vnode>, td: Option<&VThread>, name: &str) -> Result<Arc<Vnode>, Box<dyn Errno>> {
    // Populate devices.
    let fs = vn
        .fs()
        .data()
        .and_then(|v| v.downcast_ref::<DevFs>())
        .unwrap();

    fs.populate();

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
    if let Err(e) = vn.access(td, 0100) {
        return Err(Box::new(LookupError::AccessDenied(e)));
    }

    // Check name.
    if name == "." {
        return Ok(vn.clone());
    }

    let dirent = vn.data().downcast_ref::<Dirent>().unwrap();

    if name == ".." {
        let parent = match dirent.parent() {
            Some(v) => v,
            None => return Err(Box::new(LookupError::NoParent)),
        };

        return match alloc_vnode(vn.fs(), &parent) {
            Ok(v) => Ok(v),
            Err(e) => Err(Box::new(LookupError::AllocVnodeFailed(e))),
        };
    }

    // Lookup.
    let item = match dirent.find(name, None) {
        Some(v) => {
            // TODO: Implement devfs_prison_check.
            v
        }
        None => todo!("devfs lookup with non-existent file"),
    };

    match alloc_vnode(vn.fs(), &item) {
        Ok(v) => Ok(v),
        Err(e) => Err(Box::new(LookupError::AllocVnodeFailed(e))),
    }
}

/// Represents an error when [`lookup()`] is failed.
#[derive(Debug, Error)]
enum LookupError {
    #[error("file is not a directory")]
    NotDirectory,

    #[error("cannot resolve '..' on the root directory")]
    DotdotOnRoot,

    #[error("access denied")]
    AccessDenied(#[source] Box<dyn Errno>),

    #[error("file have no parent")]
    NoParent,

    #[error("cannot allocate a vnode")]
    AllocVnodeFailed(#[source] AllocVnodeError),
}

impl Errno for LookupError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::NotDirectory => ENOTDIR,
            Self::DotdotOnRoot => EIO,
            Self::AccessDenied(e) => e.errno(),
            Self::NoParent => ENOENT,
            Self::AllocVnodeFailed(e) => e.errno(),
        }
    }
}
