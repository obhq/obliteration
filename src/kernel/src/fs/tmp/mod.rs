use super::{FsOps, Mount, MountFlags, Vnode};
use crate::errno::{Errno, EINVAL};
use crate::fs::Mode;
use crate::ucred::{Gid, Uid};
use std::any::Any;
use std::collections::HashMap;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

fn mount(mnt: &mut Mount, mut opts: HashMap<String, Box<dyn Any>>) -> Result<(), Box<dyn Errno>> {
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
    let gid = if mnt.cred().real_uid() == Uid::ROOT {
        match opts.remove("gid") {
            Some(v) => *v.downcast::<Gid>().unwrap(),
            None => attrs.gid(),
        }
    } else {
        attrs.gid()
    };

    // Get UID.
    let uid = if mnt.cred().real_uid() == Uid::ROOT {
        match opts.remove("uid") {
            Some(v) => *v.downcast::<Uid>().unwrap(),
            None => attrs.uid(),
        }
    } else {
        attrs.uid()
    };

    // Get mode.
    let mode = if mnt.cred().real_uid() == Uid::ROOT {
        match opts.remove("mode") {
            Some(v) => *v.downcast::<Mode>().unwrap(),
            None => attrs.mode(),
        }
    } else {
        attrs.mode()
    };

    // Get maximum inodes.
    let inodes = match opts.remove("inodes") {
        Some(v) => *v.downcast::<usize>().unwrap(),
        None => 0,
    };

    // Get size.
    let size = match opts.remove("size") {
        Some(v) => *v.downcast::<usize>().unwrap(),
        None => 0,
    };

    // Get maximum file size.
    let file_size = match opts.remove("maxfilesize") {
        Some(v) => *v.downcast::<usize>().unwrap(),
        None => 0,
    };

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
