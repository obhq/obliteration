use super::{FsConfig, FsOps, LookupError, Mount, MountFlags, MountOpts, VPathBuf, Vnode};
use crate::errno::{Errno, EDEADLK, EINVAL, EOPNOTSUPP};
use crate::ucred::Ucred;
use bitflags::bitflags;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

mod vnode;

pub fn mount(
    _conf: &'static FsConfig,
    _cred: &Arc<Ucred>,
    _path: VPathBuf,
    parent: Option<Arc<Vnode>>,
    mut opts: MountOpts,
    flags: MountFlags,
) -> Result<Mount, Box<dyn Errno>> {
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

    let _target: VPathBuf = opts
        .remove("target")
        .or_else(|| opts.remove("from"))
        .ok_or(MountError::NoTarget)?
        .unwrap();

    todo!()
}

fn root(mnt: &Arc<Mount>) -> Arc<Vnode> {
    let null_mount: &NullFs = mnt.data().downcast_ref().unwrap();

    return null_mount.root().clone();
}

pub(super) static NULLFS_OPS: FsOps = FsOps { root };

/// An implementation of `null_mount` structure.
struct NullFs {
    root: Arc<Vnode>,          // nullm_rootvp
    lower: Option<Arc<Vnode>>, // nullm_lowervp
    flags: NullFsFlags,        // null_flags
}

impl NullFs {
    pub fn root(&self) -> &Arc<Vnode> {
        &self.root
    }

    pub fn lower(&self) -> Option<&Arc<Vnode>> {
        self.lower.as_ref()
    }

    pub fn flags(&self) -> NullFsFlags {
        self.flags
    }
}

bitflags! {
    #[derive(Clone, Copy)]
    struct NullFsFlags: u64 {}
}

struct NullNode {
    lower: Arc<Vnode>,
}

impl NullNode {
    /// See `null_nodeget` on the PS4 for a reference.
    pub fn new(mnt: &Arc<Mount>, lower: Arc<Vnode>) -> Result<Arc<Vnode>, GetNullNodeError> {
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

    #[error("target path is not specified")]
    NoTarget,

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
            MountError::LookupFailed(e) => e.errno(),
            MountError::AvoidingDeadlock => EDEADLK,
        }
    }
}

#[derive(Debug, Error)]
pub enum GetNullNodeError {}

impl Errno for GetNullNodeError {
    fn errno(&self) -> NonZeroI32 {
        todo!()
    }
}
