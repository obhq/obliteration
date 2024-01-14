use super::NullFs;
use crate::{
    errno::{Errno, EROFS},
    fs::{perm::Access, MountFlags, OpenFlags, VFile, Vnode, VnodeType, VopVector},
    process::VThread,
};
use std::{num::NonZeroI32, sync::Arc};
use thiserror::Error;

#[allow(dead_code)]
pub(super) static VNODE_OPS: VopVector = VopVector {
    default: None,
    access: Some(access),
    accessx: Some(access),
    getattr: None,
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
pub enum BypassError {}

impl Errno for BypassError {
    fn errno(&self) -> NonZeroI32 {
        todo!()
    }
}

fn lookup(vn: &Arc<Vnode>, td: Option<&VThread>, name: &str) -> Result<Arc<Vnode>, Box<dyn Errno>> {
    let null_mount: &NullFs = vn.data().downcast_ref().unwrap();

    let lower = null_mount.lower().unwrap().lookup(td, name)?;

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
