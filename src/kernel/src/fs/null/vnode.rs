use super::{GetNullNodeError, NullNode};
use crate::{
    errno::{Errno, EISDIR, EROFS},
    fs::{
        perm::Access, Mode, MountFlags, OpenFlags, VFile, Vnode, VnodeAttrs, VnodeType, VopVector,
    },
    process::VThread,
    ucred::{Gid, Uid},
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

fn setattr(vn: &Arc<Vnode>, vattr: VnodeAttrs) -> Result<VnodeAttrs, Box<dyn Errno>> {
    if vattr.uid() != Uid::VNOVAL || vattr.gid() != Gid::VNOVAL || vattr.mode() != Mode::VNOVAL {
        if vn.fs().flags().contains(MountFlags::MNT_RDONLY) {
            Err(SetAttrError::Readonly)?
        }
    }

    match vn.ty() {
        VnodeType::Directory(_) => todo!(),
        VnodeType::Character => return Err(SetAttrError::Readonly)?,
        VnodeType::File | VnodeType::Link => todo!(),
    }
}

fn getattr(vn: &Arc<Vnode>) -> Result<VnodeAttrs, Box<dyn Errno>> {
    //TODO: call null_bypass
    let fsid = vn.fs().stats().id()[0];

    Ok(VnodeAttrs::empty().with_fsid(fsid))
}

fn lookup(vn: &Arc<Vnode>, td: Option<&VThread>, name: &str) -> Result<Arc<Vnode>, Box<dyn Errno>> {
    let node: &NullNode = vn.data().downcast_ref().unwrap();

    let lower = node
        .lower()
        .lookup(td, name)
        .map_err(LookupError::LookupFromLowerFailed)?;

    let vnode = if Arc::ptr_eq(&lower, vn) {
        vn.clone()
    } else {
        NullNode::new(vn.fs(), lower)?
    };

    Ok(vnode)
}

fn open(
    vn: &Arc<Vnode>,
    _td: Option<&VThread>,
    _mode: OpenFlags,
    mut _file: Option<&mut VFile>,
) -> Result<(), Box<dyn Errno>> {
    let _mnt = vn.fs();

    todo!()
}

#[derive(Debug, Error)]
pub enum AccessError {
    #[error("mounted as readonly+")]
    Readonly,

    #[error("vnode is directory")]
    IsDirectory,
}

impl Errno for AccessError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::Readonly => EROFS,
            Self::IsDirectory => EISDIR,
        }
    }
}

#[derive(Debug, Error)]
pub enum SetAttrError {
    #[error("mounted as readonly+")]
    Readonly,
}

impl Errno for SetAttrError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::Readonly => EROFS,
        }
    }
}

#[derive(Debug, Error)]
pub enum LookupError {
    #[error("lookup failed")]
    LookupFromLowerFailed(#[source] Box<dyn Errno>),

    #[error("failed to get nullnode")]
    GetNullNodeFailed(#[from] GetNullNodeError),
}

impl Errno for LookupError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::LookupFromLowerFailed(e) => e.errno(),
            Self::GetNullNodeFailed(e) => e.errno(),
        }
    }
}
