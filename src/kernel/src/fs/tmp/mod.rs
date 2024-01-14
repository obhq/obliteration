use super::{FsOps, Mount, MountFlags, MountOpts, Vnode};
use crate::errno::{Errno, EINVAL};
use crate::fs::Mode;
use crate::ucred::{Gid, Uid};
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

#[allow(unused_variables)]
fn mount(mnt: &mut Mount, mut opts: MountOpts) -> Result<(), Box<dyn Errno>> {
    if mnt.flags().intersects(MountFlags::MNT_UPDATE) {
        return Err(Box::new(MountError::UpdateNotSupported));
    }

    // Get mount point attributes.
    let parent = mnt.parent().unwrap();
    let attrs = match parent.getattr() {
        Ok(v) => v,
        Err(e) => return Err(Box::new(MountError::GetParentAttrsFailed(e))),
    };

    // Get GID.
    let gid: Gid = if mnt.cred().real_uid() == Uid::ROOT {
        match opts.remove("gid") {
            Some(v) => v.try_into().unwrap(),
            None => attrs.gid(),
        }
    } else {
        attrs.gid()
    };

    // Get UID.
    let uid: Uid = if mnt.cred().real_uid() == Uid::ROOT {
        match opts.remove("uid") {
            Some(v) => v.try_into().unwrap(),
            None => attrs.uid(),
        }
    } else {
        attrs.uid()
    };

    // Get mode.
    let mode: Mode = if mnt.cred().real_uid() == Uid::ROOT {
        match opts.remove("mode") {
            Some(v) => v.try_into().unwrap(),
            None => attrs.mode(),
        }
    } else {
        attrs.mode()
    };

    // Get maximum inodes.
    let inodes: usize = opts.remove("inodes").map_or(0, |v| v.try_into().unwrap());

    // Get size.
    let size: usize = opts.remove("size").map_or(0, |v| v.try_into().unwrap());

    // Get maximum file size.
    let file_size: usize = opts
        .remove("maxfilesize")
        .map_or(0, |v| v.try_into().unwrap());

    todo!()
}

fn root(_: &Arc<Mount>) -> Arc<Vnode> {
    todo!()
}

/// Represents an error when [`mount()`] was failed.
#[derive(Debug, Error)]
enum MountError {
    #[error("update is not supported")]
    UpdateNotSupported,

    #[error("cannot get mount point attributes")]
    GetParentAttrsFailed(#[source] Box<dyn Errno>),
}

impl Errno for MountError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::UpdateNotSupported => EINVAL,
            Self::GetParentAttrsFailed(e) => e.errno(),
        }
    }
}

pub(super) static TMPFS_OPS: FsOps = FsOps { mount, root };
