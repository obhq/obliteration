use super::NullNode;
use crate::{
    errno::{Errno, EISDIR, EROFS},
    fs::{perm::Access, MountFlags, OpenFlags, VFile, Vnode, VnodeAttrs, VnodeType, VopVector},
    process::VThread,
};
use std::{num::NonZeroI32, sync::Arc};
use thiserror::Error;

pub(super) static VNODE_OPS: VopVector = VopVector {
    default: None,
    access: Some(access),
    accessx: Some(access),
    getattr: Some(getattr),
    lookup: Some(lookup),
    open: Some(open),
};

/// Serves as both `access` and `accessx`.
/// This function tries to mimic what calling `null_bypass` would do.
fn access(vn: &Arc<Vnode>, td: Option<&VThread>, access: Access) -> Result<(), Box<dyn Errno>> {
    if access.contains(Access::WRITE) {
        match vn.ty() {
            VnodeType::Directory(_) | VnodeType::Link | VnodeType::File => {
                if vn.fs().flags().contains(MountFlags::MNT_RDONLY) {
                    Err(AccessError::Readonly)?
                }
            }
            _ => {}
        }
    }

    let null_node: &NullNode = vn.data().downcast_ref().unwrap();

    null_node.lower().access(td, access)?;

    Ok(())
}

/// This function tries to mimic what calling `null_bypass` would do.
fn getattr(vn: &Arc<Vnode>) -> Result<VnodeAttrs, Box<dyn Errno>> {
    let null_node: &NullNode = vn.data().downcast_ref().unwrap();

    let mut attr = null_node.lower().getattr()?;

    attr.set_fsid(vn.fs().stats().id()[0]);

    Ok(attr)
}

/// This function tries to mimic what calling `null_bypass` would do.
fn lookup(vn: &Arc<Vnode>, td: Option<&VThread>, name: &str) -> Result<Arc<Vnode>, Box<dyn Errno>> {
    let null_node: &NullNode = vn.data().downcast_ref().unwrap();

    let lower = null_node.lower().lookup(td, name)?;

    let vnode = if Arc::ptr_eq(&lower, vn) {
        vn.clone()
    } else {
        NullNode::new(vn.fs(), lower)
    };

    Ok(vnode)
}

/// This function tries to mimic what calling `null_bypass` would do.
fn open(
    vn: &Arc<Vnode>,
    td: Option<&VThread>,
    mode: OpenFlags,
    mut file: Option<&mut VFile>,
) -> Result<(), Box<dyn Errno>> {
    let null_node: &NullNode = vn.data().downcast_ref().unwrap();

    // TODO: implement VOP_PROPAGATE

    null_node
        .lower()
        .open(td, mode, file)
        .map_err(OpenError::OpenFromLowerFailed)?;

    Ok(())
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
    LookupFromLowerFailed(#[from] Box<dyn Errno>),
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
