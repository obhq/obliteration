use super::{FsOps, LookupError, Mount, MountFlags, MountOpts, Vnode};
use crate::errno::{Errno, EDEADLK, EINVAL, EOPNOTSUPP};
use crate::fs::VPath;
use bitflags::bitflags;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

mod vnode;

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
        let data_constructor = |vn: &Arc<Vnode>| {
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

    let fs = mnt.fs().unwrap();

    let vnode = fs.lookup(target, None).map_err(MountError::LookupFailed)?;

    if isvnunlocked {
        todo!("nullfs_mount with isvnunlocked = true")
    }

    let node = parent_ref.data().downcast_ref::<NullNode>().unwrap();

    if Arc::ptr_eq(&vnode, &node.lower()) {
        Err(MountError::AvoidingDeadlock)?
    }

    drop(parent);

    let null_mount = NullMount::new(None, &vnode);

    let node = NullNode::get(mnt, &vnode);

    let role: Option<Box<str>> = opts.remove("role").map(|role| role.try_into().unwrap());

    if let Some("data") = role.as_deref() {
        todo!("nullfs_mount with role = data")
    }

    let null_mount = vnode.data().downcast_ref::<NullMount>().unwrap();

    if null_mount
        .lower
        .unwrap()
        .fs()
        .flags()
        .intersects(MountFlags::MNT_LOCAL)
    {
        *mnt.flags_mut() |= MountFlags::MNT_LOCAL;
    }

    mnt.set_data(Arc::new(null_mount));

    todo!()
}

fn root(mnt: &Arc<Mount>) -> Arc<Vnode> {
    let nullfs = mnt.data().unwrap().downcast_ref::<NullMount>().unwrap();

    nullfs.root.clone()
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
