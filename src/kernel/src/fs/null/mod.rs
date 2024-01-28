use super::{Filesystem, FsConfig, Mount, MountFlags, MountOpts, VPathBuf, Vnode};
use crate::errno::{Errno, EDEADLK, EOPNOTSUPP};
use crate::ucred::Ucred;
use std::num::NonZeroI32;
use std::sync::{Arc, Weak};
use thiserror::Error;

mod vnode;

/// An implementation of `null_mount` structure.
#[derive(Debug)]
struct NullFs {
    root: Arc<Vnode>, // nullm_rootvp
}

impl NullFs {
    pub fn new(root: Arc<Vnode>) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Arc<Vnode> {
        &self.root
    }
}

impl Filesystem for NullFs {
    fn root(self: Arc<Self>, mnt: &Arc<Mount>) -> Arc<Vnode> {
        self.root.clone()
    }
}

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
            todo!("nullfs_mount with MNT_UPDATE and export = true")
        } else {
            Err(MountError::NoExport)?
        }
    }

    let root = null_nodeget(parent.clone());

    let nullfs = NullFs::new(root);

    let mnt = Mount::new(conf, cred, path, Some(parent), flags, nullfs);

    Ok(mnt)
}

struct NullNode {
    null_node: Weak<Vnode>,
    lower: Arc<Vnode>,
}

impl NullNode {
    fn lower(&self) -> &Arc<Vnode> {
        &self.lower
    }
}

/// See `null_nodeget` on the PS4 for a reference.
pub(self) fn null_nodeget(lower: Arc<Vnode>) -> Arc<Vnode> {
    todo!()
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
