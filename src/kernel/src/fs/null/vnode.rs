use super::NullMount;
use crate::{
    errno::{Errno, EROFS, EINVAL},
    fs::{null::NullNode, perm::Access, MountFlags, OpenFlags, VFile, Vnode, VnodeType, VopVector, VnodeOpDesc},
    process::VThread,
};
use std::{num::NonZeroI32, sync::Arc};
use thiserror::Error;

pub(super) static VNODE_OPS: VopVector = VopVector {
    default: None,
    access: Some(access),
    accessx: Some(access),
    bypass: Some(bypass),
    getattr: None,
    lookup: Some(lookup),
    open: Some(open),
};

//Serves as both `access` and `accessx`.
fn access(vn: &Arc<Vnode>, td: Option<&VThread>, access: Access) -> Result<(), Box<dyn Errno>> {
    if access.contains(Access::WRITE) {
        match vn.ty() {
            VnodeType::Directory(_) | VnodeType::Link | VnodeType::Reg => {
                if vn.fs().flags().contains(MountFlags::MNT_RDONLY) {
                    Err(AccessError::Readonly)?
                }
            }
            _ => {}
        }
    }

    todo!();
    //bypass(desc)
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

fn bypass(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    let mut flags = desc.flags();

    let vnodes: [Option<&Arc<Vnode>>; VnodeOpDesc::VDESC_MAX_VPS] = [None; VnodeOpDesc::VDESC_MAX_VPS];

    for (i, offset) in desc.offsets().iter().enumerate() {
        flags >>= unsafe {i.try_into().unwrap_unchecked() } ;

        if *offset == -1 {
            break;
        }

        vnodes[i] =
    }

    let result = if let Some(vnode) = vnodes[0] {
        desc.vcall()
    } else {
        Err(BypassError::NoMap(desc.name()).into())
    };

    todo!();

    result
}

#[derive(Debug, Error)]
pub enum BypassError {
    #[error("no map for {0}")]
    NoMap(&'static str)
}

impl Errno for BypassError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::NoMap(_) => EINVAL,
        }
    }
}

fn lookup(vn: &Arc<Vnode>, td: Option<&VThread>, name: &str) -> Result<Arc<Vnode>, Box<dyn Errno>> {
    let null_mount: &NullMount = vn.data().downcast_ref().unwrap();

    let lower = null_mount.lower().unwrap().lookup(td, name)?;

    let vnode = if Arc::ptr_eq(&lower, vn) {
        vn.clone()
    } else {
        NullNode::get(vn.fs(), &lower)
    };

    Ok(vnode)
}

fn open(
    vn: &Arc<Vnode>,
    td: Option<&VThread>,
    mode: OpenFlags,
    mut file: Option<&mut VFile>,
) -> Result<(), Box<dyn Errno>> {
    todo!()
}
