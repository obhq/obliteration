use super::{FsConfig, FsOps, LookupError, Mount, MountFlags, MountOpts, VPathBuf, Vnode};
use crate::errno::{Errno, EDEADLK, EINVAL, EOPNOTSUPP};
use crate::fs::VPath;
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
    _parent: Option<Arc<Vnode>>,
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

    let target: Box<str> = opts
        .remove("target")
        .or_else(|| opts.remove("from"))
        .ok_or(MountError::NoTarget)?
        .unwrap();

    if target.is_empty() {
        Err(MountError::EmptyTarget)?;
    }

    let _target: &VPath = target.as_ref().try_into().unwrap();

    todo!()
}

fn root(_mnt: &Arc<Mount>) -> Arc<Vnode> {
    todo!()
}

pub(super) static NULLFS_OPS: FsOps = FsOps { root };

#[derive(Debug, Error)]
#[allow(dead_code)]
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
#[allow(dead_code)]
struct NullFs {
    root: Arc<Vnode>,          // nullm_rootvp
    lower: Option<Arc<Vnode>>, // nullm_lowervp
    flags: NullFsFlags,        // null_flags
}

#[allow(dead_code)]
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

#[allow(dead_code)]
struct NullNode {
    lower: Arc<Vnode>,
}

#[allow(dead_code)]
impl NullNode {
    fn lower(&self) -> &Arc<Vnode> {
        &self.lower
    }
}
