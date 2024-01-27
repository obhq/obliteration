use super::NullNode;
use crate::{
    errno::{Errno, EISDIR, EROFS},
    fs::{perm::Access, MountFlags, OpenFlags, VFile, Vnode, VnodeAttrs, VnodeType},
    process::VThread,
};
use std::{num::NonZeroI32, sync::Arc};
use thiserror::Error;

#[derive(Debug)]
struct VnodeBackend {
    lower: Arc<Vnode>,
}

impl crate::fs::VnodeBackend for VnodeBackend {
    fn accessx(
        self: Arc<Self>,
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
    fn getattr(self: Arc<Self>, vn: &Arc<Vnode>) -> Result<VnodeAttrs, Box<dyn Errno>> {
        let attr = self
            .lower
            .getattr()
            .map_err(GetAttrError::GetAttrFromLowerFailed)?;

        let fsid = vn.fs().stats().id()[0];

        Ok(attr.with_fsid(fsid))
    }

    /// This function tries to mimic what calling `null_bypass` would do.
    fn lookup(
        self: Arc<Self>,
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
            NullNode::get(vn.fs(), lower)
        };

        Ok(vnode)
    }

    /// This function tries to mimic what calling `null_bypass` would do.
    fn open(
        self: Arc<Self>,
        vn: &Arc<Vnode>,
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

#[derive(Debug, Error)]
pub enum AccessError {
    #[error("mounted as readonly+")]
    Readonly,

    #[error("vnode is directory")]
    IsDirectory,

    #[error("access from lower vnode failed")]
    AccessFromLowerFailed(#[from] Box<dyn Errno>),
}

impl Errno for AccessError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::Readonly => EROFS,
            Self::IsDirectory => EISDIR,
            Self::AccessFromLowerFailed(e) => e.errno(),
        }
    }
}

#[derive(Debug, Error)]
pub enum GetAttrError {
    #[error("getattr from lower vnode failed")]
    GetAttrFromLowerFailed(#[from] Box<dyn Errno>),
}

impl Errno for GetAttrError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::GetAttrFromLowerFailed(e) => e.errno(),
        }
    }
}

#[derive(Debug, Error)]
pub enum LookupError {
    #[error("lookup failed")]
    LookupFromLowerFailed(#[source] Box<dyn Errno>),
}

impl Errno for LookupError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::LookupFromLowerFailed(e) => e.errno(),
        }
    }
}

#[derive(Debug, Error)]
pub enum OpenError {
    #[error("open from lower vnode failed")]
    OpenFromLowerFailed(#[source] Box<dyn Errno>),
}

impl Errno for OpenError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::OpenFromLowerFailed(e) => e.errno(),
        }
    }
}
