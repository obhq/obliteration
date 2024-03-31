use self::file::{HostFile, HostId};
use self::vnode::VnodeBackend;
use super::{
    Filesystem, Fs, FsConfig, Mount, MountFlags, MountOpt, MountOpts, MountSource, VPathBuf, Vnode,
    VnodeType,
};
use crate::errno::{Errno, EIO};
use crate::ucred::Ucred;
use macros::Errno;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Weak};
use thiserror::Error;

mod file;
mod vnode;

pub fn mount(
    _: Option<&Arc<Fs>>,
    conf: &'static FsConfig,
    cred: &Arc<Ucred>,
    path: VPathBuf,
    parent: Option<Arc<Vnode>>,
    mut opts: MountOpts,
    flags: MountFlags,
) -> Result<Mount, Box<dyn Errno>> {
    // Check mount flags.
    if flags.intersects(MountFlags::MNT_UPDATE) {
        todo!("update HostFS mounting");
    }

    // Get source name.
    let from: MountOpt = opts.remove("from").unwrap();
    let source = match from {
        MountOpt::Str(v) => MountSource::Driver(Cow::Owned(v.into_string())),
        MountOpt::VPath(v) => MountSource::Path(v),
        _ => unreachable!(),
    };

    // Open root directory.
    let root: PathBuf = opts.remove("ob:root").unwrap();
    let root = match HostFile::root(&root) {
        Ok(v) => v,
        Err(e) => return Err(Box::new(MountError::OpenRootFailed(root, e))),
    };

    // Set mount data.
    Ok(Mount::new(
        conf,
        cred,
        source,
        path,
        parent,
        flags | MountFlags::MNT_LOCAL,
        HostFs {
            root: Arc::new(root),
            actives: Mutex::default(),
        },
    ))
}

/// Implementation of [`Filesystem`] to mount a directory from the host.
///
/// We subtitute `exfatfs` and `pfs` with this because root FS on the PS4 is exFAT and game data is
/// PFS. That mean we must report this either as `exfatfs` or `pfs` otherwise it might be unexpected
/// by the PS4.
#[derive(Debug)]
pub struct HostFs {
    root: Arc<HostFile>,
    actives: Mutex<HashMap<HostId, Weak<Vnode>>>,
}

impl Filesystem for HostFs {
    fn root(self: Arc<Self>, mnt: &Arc<Mount>) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        let vnode = self.get_vnode(mnt, &self.root)?;
        Ok(vnode)
    }
}

impl HostFs {
    fn get_vnode(
        self: &Arc<HostFs>,
        mnt: &Arc<Mount>,
        file: &Arc<HostFile>,
    ) -> Result<Arc<Vnode>, GetVnodeError> {
        // Get file ID.
        let id = match file.id() {
            Ok(v) => v,
            Err(e) => return Err(GetVnodeError::GetFileIdFailed(e)),
        };

        // Check if active.
        let mut actives = self.actives.lock().unwrap();

        if let Some(v) = actives.get(&id).and_then(|v| v.upgrade()) {
            return Ok(v);
        }

        // Get vnode type. Beware of deadlock here.
        let ty = match file.is_directory() {
            Ok(true) => VnodeType::Directory(Arc::ptr_eq(file, &self.root)),
            Ok(false) => VnodeType::File,
            Err(e) => return Err(GetVnodeError::GetFileTypeFailed(e)),
        };

        // Allocate a new vnode.
        let vn = Vnode::new(
            mnt,
            ty,
            mnt.config().name,
            VnodeBackend::new(self.clone(), file.clone()),
        );

        actives.insert(id, Arc::downgrade(&vn));

        Ok(vn)
    }
}

/// Represents an error when [`mount()`] fails.
#[derive(Debug, Error, Errno)]
enum MountError {
    #[error("couldn't open {0} as a root directory")]
    #[errno(EIO)]
    OpenRootFailed(PathBuf, #[source] std::io::Error),
}
/// Represents an error when [`get_vnode()`] fails.
#[derive(Debug, Error, Errno)]
enum GetVnodeError {
    #[error("couldn't get file identifier")]
    #[errno(EIO)]
    GetFileIdFailed(#[source] std::io::Error),

    #[error("cannot determine file type")]
    #[errno(EIO)]
    GetFileTypeFailed(#[source] std::io::Error),
}
