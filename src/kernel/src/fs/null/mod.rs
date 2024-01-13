use super::{FsOps, LookupError, Mount, MountFlags, MountOpts, Vnode};
use crate::errno::{Errno, EDEADLK, EINVAL, EOPNOTSUPP};
use crate::fs::VPath;
use bitflags::bitflags;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

mod vnode;

fn mount(mnt: &mut Mount, mut opts: MountOpts) -> Result<(), Box<dyn Errno>> {
    let flags = mnt.flags();

    if flags.intersects(MountFlags::MNT_ROOTFS) {
        Err(MountError::RootFs)?;
    }

    if flags.intersects(MountFlags::MNT_UPDATE) {
        if opts
            .remove("export")
            .is_some_and(|opt| opt.try_into().unwrap())
        {
            //noop
            return Ok(());
        } else {
            Err(MountError::NoExport)?
        }
    }

    drop(flags);

    let target: Box<str> = opts
        .remove("target")
        .or_else(|| opts.remove("from"))
        .ok_or_else(|| MountError::NoTarget)?
        .try_into()
        .unwrap();

    if target.is_empty() {
        Err(MountError::EmptyTarget)?;
    }

    let target: &VPath = target.as_ref().try_into().unwrap();

    let parent = mnt.parent();
    let parent_ref = parent.as_ref().expect("No parent");

    let isvnunlocked = if std::ptr::eq(parent_ref.op(), &vnode::VNODE_OPS) {
        todo!();

        #[allow(unreachable_code)]
        true
    } else {
        false
    };

    todo!()
}

fn root(mnt: &Arc<Mount>) -> Arc<Vnode> {
    todo!()
}

pub(super) static NULLFS_OPS: FsOps = FsOps { mount, root };

#[derive(Debug, Error)]
enum MountError {
    #[error("mounting as root FS is not supported")]
    RootFs,

    #[error("update mount is not supported without export option")]
    NoExport,

    #[error("target path is not specified")]
    NoTarget,

    #[error("target path is empty")]
    EmptyTarget,

    #[error("lookup failed")]
    LookupFailed(#[source] LookupError),

    #[error("avoiding deadlock")]
    AvoidingDeadlock,
}

impl Errno for MountError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            MountError::RootFs => EOPNOTSUPP,
            MountError::NoExport => EOPNOTSUPP,
            MountError::NoTarget => EINVAL,
            MountError::EmptyTarget => EINVAL,
            MountError::LookupFailed(e) => e.errno(),
            MountError::AvoidingDeadlock => EDEADLK,
        }
    }
}

/// An implementation of `null_mount` structure.
struct NullMount {
    root: Arc<Vnode>,          // nullm_rootvp
    lower: Option<Arc<Vnode>>, // nullm_lowervp
    flags: NullMountFlags,     // null_flags
}

impl NullMount {
    fn new(lower: Option<&Arc<Vnode>>, root: &Arc<Vnode>) -> Self {
        Self {
            root: root.clone(),
            lower: lower.cloned(),
            flags: NullMountFlags::empty(),
        }
    }

    pub fn lower(&self) -> Option<&Arc<Vnode>> {
        self.lower.as_ref()
    }
}

bitflags! {
    struct NullMountFlags: u64 {}
}

struct NullNode {
    lower: Arc<Vnode>,
}

impl NullNode {
    fn lower(&self) -> &Arc<Vnode> {
        &self.lower
    }

    /// See `null_nodeget` on the PS4 for reference.
    fn get(mnt: &Arc<Mount>, lower: &Arc<Vnode>) -> Arc<Vnode> {
        let data_constructor = |_: &Arc<Vnode>| {
            Arc::new(NullNode {
                lower: lower.clone(),
            })
        };

        let vnode = unsafe {
            Vnode::new_with(
                mnt,
                *lower.ty(),
                "null",
                &vnode::VNODE_OPS,
                data_constructor,
            )
        };

        // TODO: Implement insmntque1.

        vnode
    }
}
