use super::{FsOps, Mount, MountFlags, MountOpts, Vnode};
use crate::errno::{Errno, EINVAL};
use std::any::Any;
use std::collections::HashMap;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

fn mount(mnt: &mut Mount, _: MountOpts) -> Result<(), Box<dyn Errno>> {
    if mnt.flags().intersects(MountFlags::MNT_UPDATE) {
        return Err(Box::new(MountError::UpdateNotSupported));
    }

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
}

impl Errno for MountError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::UpdateNotSupported => EINVAL,
        }
    }
}

pub(super) static TMPFS_OPS: FsOps = FsOps { mount, root };
