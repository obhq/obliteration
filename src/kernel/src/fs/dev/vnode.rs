use super::dirent::Dirent;
use super::{alloc_vnode, AllocVnodeError, Cdev, DevFs};
use crate::errno::{Errno, EIO, ENOENT, ENOTDIR, ENXIO};
use crate::fs::{check_access, Access, OpenFlags, VFile, Vnode, VnodeAttrs, VnodeType};
use crate::process::VThread;
use macros::Errno;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

/// An implementation of [`crate::fs::VnodeBackend`] for devfs.
///
/// This implementation merge `devfs_vnodeops` and `devfs_specops` together.
#[derive(Debug)]
pub struct VnodeBackend {
    fs: Arc<DevFs>,
    dirent: Arc<Dirent>,
}

impl VnodeBackend {
    pub fn new(fs: Arc<DevFs>, dirent: Arc<Dirent>) -> Self {
        Self { fs, dirent }
    }
}

impl crate::fs::VnodeBackend for VnodeBackend {
    fn access(
        self: Arc<Self>,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        mode: Access,
    ) -> Result<(), Box<dyn Errno>> {
        // Get dirent.
        let mut dirent = self.dirent.clone();
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
        let (fuid, fgid, fmode) = {
            let uid = dirent.uid();
            let gid = dirent.gid();
            let mode = dirent.mode();

            (*uid, *gid, *mode)
        };

        // Check access.
        let err = match check_access(cred, fuid, fgid, fmode, mode, is_dir) {
            Ok(_) => return Ok(()),
            Err(e) => e,
        };

        // TODO: Check if file is a controlling terminal.
        Err(Box::new(err))
    }

    fn getattr(self: Arc<Self>, vn: &Arc<Vnode>) -> Result<VnodeAttrs, Box<dyn Errno>> {
        // Populate devices.
        self.fs.populate();

        // Get dirent.
        let mut dirent = self.dirent.clone();

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
            VnodeType::Link => todo!(), /* TODO: strlen(dirent.de_symlink) */
            _ => 0,
        };

        todo!()
    }

    fn lookup(
        self: Arc<Self>,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        name: &str,
    ) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        // Populate devices.
        self.fs.populate();

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
        match name {
            "." => Ok(vn.clone()),
            ".." => {
                let parent = match self.dirent.parent() {
                    Some(v) => v,
                    None => return Err(Box::new(LookupError::NoParent)),
                };

                match alloc_vnode(self.fs.clone(), vn.fs(), parent) {
                    Ok(v) => Ok(v),
                    Err(e) => Err(Box::new(LookupError::AllocVnodeFailed(e))),
                }
            }
            _ => {
                // Lookup.
                let item = match self.dirent.find(name, None) {
                    Some(v) => {
                        // TODO: Implement devfs_prison_check.
                        v
                    }
                    None => todo!("devfs lookup with non-existent file"),
                };

                match alloc_vnode(self.fs.clone(), vn.fs(), item) {
                    Ok(v) => Ok(v),
                    Err(e) => Err(Box::new(LookupError::AllocVnodeFailed(e))),
                }
            }
        }
    }

    fn open(
        self: Arc<Self>,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        mode: OpenFlags,
        mut file: Option<&mut VFile>,
    ) -> Result<(), Box<dyn Errno>> {
        if !vn.is_character() {
            return Ok(());
        }

        // Not sure why FreeBSD check if vnode is VBLK because all of vnode here always be VCHR.
        let dev = vn.item().unwrap().downcast::<Cdev>().unwrap();
        let sw = dev.sw();

        if file.is_none() && sw.fdopen().is_some() {
            return Err(Box::new(OpenError::NeedFile));
        }

        // Execute switch handler.
        match sw.fdopen() {
            Some(f) => f(&dev, mode, td, file.as_deref_mut())?,
            None => sw.open().unwrap()(&dev, mode, 0x2000, td)?,
        };

        // Set file OP.
        let file = match file {
            Some(v) => v,
            None => return Ok(()),
        };

        // TODO: Implement remaining logics from the PS4.
        Ok(())
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

/// Represents an error when [`open()`] is failed.
#[derive(Debug, Error, Errno)]
enum OpenError {
    #[error("destination file is required")]
    #[errno(ENXIO)]
    NeedFile,
}
