use super::dirent::Dirent;
use super::{alloc_vnode, AllocVnodeError, Cdev, DevFs};
use crate::errno::{Errno, EIO, ENOENT, ENOTDIR, ENXIO};
use crate::fs::{
    check_access, Access, OpenFlags, VFile, Vnode, VnodeAttrs, VnodeType, VopVector,
    DEFAULT_VNODEOPS,
};
use crate::process::VThread;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

pub static VNODE_OPS: VopVector = VopVector {
    default: Some(&DEFAULT_VNODEOPS),
    access: Some(access),
    accessx: None,
    getattr: Some(getattr),
    lookup: Some(lookup),
    open: None,
};

pub static CHARACTER_OPS: VopVector = VopVector {
    default: Some(&DEFAULT_VNODEOPS),
    access: Some(access),
    accessx: None,
    getattr: Some(getattr),
    lookup: None,
    open: Some(open),
};

fn access(vn: &Arc<Vnode>, td: Option<&VThread>, access: Access) -> Result<(), Box<dyn Errno>> {
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

fn getattr(vn: &Arc<Vnode>) -> Result<VnodeAttrs, Box<dyn Errno>> {
    // Populate devices.
    let fs = vn.fs().data().downcast_ref::<DevFs>().unwrap();

    fs.populate();

    // Get dirent.
    let mut dirent = vn.data().clone().downcast::<Dirent>().unwrap();

    if vn.is_directory() {
        if let Some(v) = dirent.dir() {
            // Is it possible the parent will be gone here?
            dirent = v.upgrade().unwrap();
        }
    }

    // Atomic get attributes.
    let uid = dirent.uid();
    let gid = dirent.gid();
    let mode = dirent.mode();
    let size = match vn.ty() {
        VnodeType::Directory(_) => 512,
        VnodeType::Character | VnodeType::File => 0,
    };

    Ok(VnodeAttrs::new(*uid, *gid, *mode, size))
}

fn lookup(vn: &Arc<Vnode>, td: Option<&VThread>, name: &str) -> Result<Arc<Vnode>, Box<dyn Errno>> {
    // Populate devices.
    let fs = vn.fs().data().downcast_ref::<DevFs>().unwrap();

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
    vn.access(td, Access::EXEC)
        .map_err(LookupError::AccessDenied)?;

    // Check name.
    if name == "." {
        return Ok(vn.clone());
    }

    let dirent = vn.data().downcast_ref::<Dirent>().unwrap();

    if name == ".." {
        let parent = dirent.parent().ok_or(LookupError::NoParent)?;

        let vnode = alloc_vnode(vn.fs(), &parent).map_err(LookupError::AllocVnodeFailed)?;

        return Ok(vnode);
    }

    // Lookup.
    let item = match dirent.find(name, None) {
        Some(v) => {
            // TODO: Implement devfs_prison_check.
            v
        }
        None => todo!("devfs lookup with non-existent file"),
    };

    let vnode = alloc_vnode(vn.fs(), &item).map_err(LookupError::AllocVnodeFailed)?;

    return Ok(vnode);
}

fn open(
    vn: &Arc<Vnode>,
    td: Option<&VThread>,
    mode: OpenFlags,
    mut file: Option<&mut VFile>,
) -> Result<(), Box<dyn Errno>> {
    // Not sure why FreeBSD check if vnode is VBLK because all of vnode here always be VCHR.
    let dev = vn.item().unwrap().downcast::<Cdev>().unwrap();
    let sw = dev.sw();

    assert!(vn.is_character());

    if file.is_none() && sw.fdopen().is_some() {
        return Err(Box::new(OpenError::NeedFile));
    }

    // Execute switch handler.
    match sw.fdopen() {
        Some(fdopen) => fdopen(&dev, mode, td, file.as_mut().map(|f| &mut **f))?,
        None => sw.open().unwrap()(&dev, mode, 0x2000, td)?,
    };

    // Set file OP.
    let Some(file) = file else {
        return Ok(());
    };

    // TODO: Implement remaining logics from the PS4.
    Ok(())
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

/// Represents an error when [`open()`] is failed.
#[derive(Debug, Error)]
enum OpenError {
    #[error("destination file is required")]
    NeedFile,
}

impl Errno for OpenError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::NeedFile => ENXIO,
        }
    }
}
