use super::{FsConfig, FsOps, Mount, MountFlags, MountOpts, VPathBuf, Vnode};
use crate::errno::{Errno, EDEADLK, EOPNOTSUPP};
use crate::ucred::Ucred;
use std::num::NonZeroI32;
use std::sync::{Arc, Weak};
use thiserror::Error;

mod vnode;

pub fn mount(
    conf: &'static FsConfig,
    cred: &Arc<Ucred>,
    path: VPathBuf,
    parent: Option<Arc<Vnode>>,
    mut opts: MountOpts,
    flags: MountFlags,
) -> Result<Mount, Box<dyn Errno>> {
    let parent = parent.expect("No parent vnode provided to nullfs");

    if flags.intersects(MountFlags::MNT_ROOTFS) {
        Err(MountError::RootFs)?;
    }

    if flags.intersects(MountFlags::MNT_UPDATE) {
        if opts.remove("export").is_some_and(|opt| opt.unwrap()) {
            todo!("null_mount with MNT_UPDATE and export = true")
        } else {
            Err(MountError::NoExport)?
        }
    }

    let mnt = Mount::new_with_data_fn(
        conf,
        &NULLFS_OPS,
        cred,
        path,
        Some(parent.clone()),
        flags,
        |mnt| {
            let vn = NullNode::new(mnt, parent);

            NullFs::new(&vn)
        },
    );

    todo!()
}

fn root(mnt: &Arc<Mount>) -> Arc<Vnode> {
    let nullfs: &NullFs = mnt.data().downcast_ref().unwrap();

    nullfs.root().clone()
}

pub(super) static NULLFS_OPS: FsOps = FsOps { root };

/// An implementation of `null_mount` structure.
struct NullFs {
    root: Arc<Vnode>, // nullm_rootvp
}

impl NullFs {
    pub fn new(root: &Arc<Vnode>) -> Arc<Self> {
        Arc::new(Self { root: root.clone() })
    }

    pub fn root(&self) -> &Arc<Vnode> {
        &self.root
    }
}

struct NullNode {
    null_vnode: Weak<Vnode>,
    lower: Arc<Vnode>,
}

impl NullNode {
    /// See `null_nodeget` on the PS4 for a reference.
    pub fn new(mnt: &Arc<Mount>, lower: Arc<Vnode>) -> Arc<Vnode> {
        todo!()
    }

    fn lower(&self) -> &Arc<Vnode> {
        &self.lower
    }
}

#[derive(Debug, Error)]
enum MountError {
    #[error("mounting as root FS is not supported")]
    RootFs,

    #[error("update mount is not supported without export option")]
    NoExport,

    #[error("avoiding deadlock")]
    AvoidingDeadlock,
}

impl Errno for MountError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            MountError::RootFs => EOPNOTSUPP,
            MountError::NoExport => EOPNOTSUPP,
            MountError::AvoidingDeadlock => EDEADLK,
        }
    }
}
