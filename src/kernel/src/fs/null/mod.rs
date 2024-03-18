use self::vnode::null_nodeget;
use super::{
    Filesystem, Fs, FsConfig, LookupError, Mount, MountFlags, MountOpts, MountSource, VPathBuf,
    Vnode,
};
use crate::errno::{Errno, EDEADLK, EOPNOTSUPP};
use crate::ucred::Ucred;
use macros::Errno;
use std::sync::Arc;
use thiserror::Error;

mod hash;
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
    fn root(self: Arc<Self>, mnt: &Arc<Mount>) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        let vnode = null_nodeget(mnt, &self.lower)?;

        Ok(vnode)
    }
}

pub fn mount(
    fs: Option<&Arc<Fs>>,
    conf: &'static FsConfig,
    cred: &Arc<Ucred>,
    path: VPathBuf,
    _: Option<Arc<Vnode>>,
    mut opts: MountOpts,
    flags: MountFlags,
) -> Result<Mount, Box<dyn Errno>> {
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
    let target: VPathBuf =
        opts.remove_or_else("target", || todo!("nullfs_mount without target option"));

    let lower = fs
        .unwrap()
        .lookup(&path, true, None)
        .map_err(MountError::LookupTargetVnode)?;

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

    #[error("couldn't lookup target vnode")]
    LookupTargetVnode(#[source] LookupError),
}
