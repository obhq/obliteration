use super::NullNode;
use crate::{
    errno::{Errno, EROFS},
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

//Serves as both `access` and `accessx`.
fn access(vn: &Arc<Vnode>, _td: Option<&VThread>, access: Access) -> Result<(), Box<dyn Errno>> {
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

    todo!();
}

fn getattr(vn: &Arc<Vnode>) -> Result<VnodeAttrs, Box<dyn Errno>> {
    todo!()
}

fn lookup(vn: &Arc<Vnode>, td: Option<&VThread>, name: &str) -> Result<Arc<Vnode>, Box<dyn Errno>> {
    let node: &NullNode = vn.data().downcast_ref().unwrap();

    let lower = node
        .lower()
        .lookup(td, name)
        .map_err(LookupFromLowerFailed::LookupFailed)?;

    let vnode = if Arc::ptr_eq(&lower, vn) {
        vn.clone()
    } else {
        todo!();
    };

    Ok(vnode)
}

fn open(
    _vn: &Arc<Vnode>,
    _td: Option<&VThread>,
    _mode: OpenFlags,
    mut _file: Option<&mut VFile>,
) -> Result<(), Box<dyn Errno>> {
    todo!()
}

#[derive(Debug, Error)]
pub enum AccessError {
    #[error("mounted as readonly+")]
    Readonly,
}

impl Errno for AccessError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::Readonly => EROFS,
        }
    }
}

#[derive(Debug, Error)]
pub enum LookupFromLowerFailed {
    #[error("lookup failed")]
    LookupFailed(#[source] Box<dyn Errno>),
}

impl Errno for LookupFromLowerFailed {
    fn errno(&self) -> NonZeroI32 {
        todo!()
    }
}
