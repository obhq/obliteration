use crate::{
    errno::{Errno, EISDIR, EROFS},
    fs::{perm::Access, Mount, MountFlags, OpenFlags, VFile, Vnode, VnodeAttrs, VnodeType},
    process::VThread,
};
use macros::Errno;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
struct VnodeBackend {
    lower: Arc<Vnode>,
}

impl crate::fs::VnodeBackend for VnodeBackend {
    fn accessx(
        &self,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        mode: Access,
    ) -> Result<(), Box<dyn Errno>> {
        if mode.contains(Access::WRITE) {
            match vn.ty() {
                VnodeType::Directory(_) | VnodeType::Link | VnodeType::File => {
                    if vn.fs().flags().contains(MountFlags::MNT_RDONLY) {
                        Err(AccessError::Readonly)?
                    }
                }
                _ => {}
            }
        }

        self.lower
            .access(td, mode)
            .map_err(AccessError::AccessFromLowerFailed)?;

        Ok(())
    }

    /// This function tries to mimic what calling `null_bypass` would do.
    fn getattr(&self, vn: &Arc<Vnode>) -> Result<VnodeAttrs, Box<dyn Errno>> {
        let mut attr = self
            .lower
            .getattr()
            .map_err(GetAttrError::GetAttrFromLowerFailed)?;

        let fsid = vn.fs().stats().id()[0];

        attr.set_fsid(fsid);

        Ok(attr)
    }

    /// This function tries to mimic what calling `null_bypass` would do.
    fn lookup(
        &self,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        name: &str,
    ) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        let lower = self
            .lower
            .lookup(td, name)
            .map_err(LookupError::LookupFromLowerFailed)?;

        let vnode = if Arc::ptr_eq(&lower, vn) {
            vn.clone()
        } else {
            null_nodeget(vn.fs(), lower)?
        };

        Ok(vnode)
    }

    /// This function tries to mimic what calling `null_bypass` would do.
    fn open(
        &self,
        _: &Arc<Vnode>,
        td: Option<&VThread>,
        mode: OpenFlags,
        file: Option<&mut VFile>,
    ) -> Result<(), Box<dyn Errno>> {
        self.lower
            .open(td, mode, file)
            .map_err(OpenError::OpenFromLowerFailed)?;

        Ok(())
    }
}

/// See `null_nodeget` on the PS4 for a reference.
pub(super) fn null_nodeget(
    mnt: &Arc<Mount>,
    lower: Arc<Vnode>,
) -> Result<Arc<Vnode>, NodeGetError> {
    todo!()
}

#[derive(Debug, Error, Errno)]
pub enum AccessError {
    #[error("mounted as readonly")]
    #[errno(EROFS)]
    Readonly,

    #[error("vnode is directory")]
    #[errno(EISDIR)]
    IsDirectory,

    #[error("access from lower vnode failed")]
    AccessFromLowerFailed(#[from] Box<dyn Errno>),
}

#[derive(Debug, Error, Errno)]
pub enum GetAttrError {
    #[error("getattr from lower vnode failed")]
    GetAttrFromLowerFailed(#[from] Box<dyn Errno>),
}

#[derive(Debug, Error, Errno)]
pub enum LookupError {
    #[error("lookup from lower vnode failed")]
    LookupFromLowerFailed(#[source] Box<dyn Errno>),
}

#[derive(Debug, Error, Errno)]
pub enum OpenError {
    #[error("open from lower vnode failed")]
    OpenFromLowerFailed(#[source] Box<dyn Errno>),
}

#[derive(Debug, Error, Errno)]
pub(super) enum NodeGetError {}
