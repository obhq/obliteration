use super::{FsOps, Mount, MountFlags, Mounts, Vnode, VnodeType};
use crate::errno::{Errno, EOPNOTSUPP};
use std::any::Any;
use std::collections::HashMap;
use std::num::NonZeroI32;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use thiserror::Error;

pub(super) mod console;
pub(super) mod deci_tty6;
pub(super) mod dipsw;
pub(super) mod dmem0;
pub(super) mod dmem1;
pub(super) mod dmem2;

/// An implementation of `devfs_mount` structure.
pub struct DevFs {
    idx: u32,  // dm_idx
    hold: i32, // dm_holdcnt
}

fn mount(
    mounts: &Mounts,
    mount: &mut Mount,
    _: HashMap<String, Box<dyn Any>>,
) -> Result<(), Box<dyn Errno>> {
    // Check mount flags.
    let mut flags = mount.flags_mut();

    if flags.intersects(MountFlags::MNT_ROOTFS) {
        return Err(Box::new(MountError::RootFs));
    } else if flags.intersects(MountFlags::MNT_UPDATE) {
        return Err(Box::new(MountError::Update));
    }

    flags.set(MountFlags::MNT_LOCAL, true);

    drop(flags);

    // Set mount data.
    let idx = UNR.fetch_add(1, Ordering::Relaxed);

    mount.set_data(Arc::new(DevFs {
        idx: idx.try_into().unwrap(),
        hold: 1,
    }));

    mounts.set_id(mount);

    Ok(())
}

fn root(_: &Mount) -> Arc<Vnode> {
    // TODO: Check what the PS4 is doing here.
    Arc::new(Vnode::new(Some(VnodeType::Directory { mount: None })))
}

/// Represents an error when [`mount`] is failed.
#[derive(Debug, Error)]
enum MountError {
    #[error("mounting as root FS is not supported")]
    RootFs,

    #[error("update mounting is not supported")]
    Update,
}

impl Errno for MountError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::RootFs | Self::Update => EOPNOTSUPP,
        }
    }
}

pub(super) static DEVFS_OPS: FsOps = FsOps { mount, root };
static UNR: AtomicI32 = AtomicI32::new(0); // TODO: Use a proper implementation.
