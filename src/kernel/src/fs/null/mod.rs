use self::vnode::null_nodeget;
use super::{Filesystem, FsConfig, Mount, MountFlags, MountOpts, MountSource, VPathBuf, Vnode};
use crate::errno::{Errno, EDEADLK, EOPNOTSUPP};
use crate::ucred::Ucred;
use macros::Errno;
use std::sync::Arc;
use thiserror::Error;

mod vnode;

/// An implementation of `null_mount` structure.
#[derive(Debug)]
struct NullFs {
    lower: Arc<Vnode>,
}

impl NullFs {
    pub fn lower(&self) -> &Arc<Vnode> {
        &self.lower
    }
}

impl Filesystem for NullFs {
    fn root(self: Arc<Self>, mnt: &Arc<Mount>) -> Arc<Vnode> {
        null_nodeget(mnt, self.lower.clone())
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
    let lower = parent.expect("No parent vnode provided to nullfs");

    if flags.intersects(MountFlags::MNT_ROOTFS) {
        Err(MountError::RootFs)?;
    }

    if flags.intersects(MountFlags::MNT_UPDATE) {
        if let Some(true) = opts.remove("export") {
            todo!("nullfs_mount with MNT_UPDATE and export = true")
        } else {
            Err(MountError::NoExport)?
        }
    }

    // Get target path.
    let target: VPathBuf = opts
        .remove("target")
        .unwrap_or_else(|| todo!("nullfs_mount without target option"));

    Ok(Mount::new(
        conf,
        cred,
        MountSource::Path(target),
        path,
        Some(lower.clone()),
        flags,
        NullFs { lower },
    ))
}

#[derive(Debug, Error, Errno)]
enum MountError {
    #[error("mounting as root FS is not supported")]
    #[errno(EOPNOTSUPP)]
    RootFs,

    #[error("update mount is not supported without export option")]
    #[errno(EOPNOTSUPP)]
    NoExport,

    #[error("avoiding deadlock")]
    #[errno(EDEADLK)]
    AvoidingDeadlock,
}
