use self::file::{HostFile, HostId};
use self::vnode::VnodeBackend;
use super::{Filesystem, FsConfig, Mount, MountFlags, MountOpts, VPathBuf, Vnode, VnodeType};
use crate::errno::{Errno, EIO};
use crate::ucred::Ucred;
use macros::Errno;
use param::Param;
use std::collections::HashMap;
use std::fs::create_dir;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Weak};
use thiserror::Error;

mod file;
mod vnode;

/// Mount data for host FS.
///
/// We subtitute `exfatfs` with this because the root FS on the PS4 is exFAT. That mean we must
/// report this as `exfatfs` otherwise it might be unexpected by the PS4.
#[derive(Debug)]
pub struct HostFs {
    root: Arc<HostFile>,
    actives: Mutex<HashMap<HostId, Weak<Vnode>>>,
}

pub fn mount(
    conf: &'static FsConfig,
    cred: &Arc<Ucred>,
    path: VPathBuf,
    parent: Option<Arc<Vnode>>,
    mut opts: MountOpts,
    flags: MountFlags,
) -> Result<Mount, Box<dyn Errno>> {
    // Check mount flags.
    if !flags.intersects(MountFlags::MNT_ROOTFS) {
        todo!("mounting host FS on non-root");
    } else if flags.intersects(MountFlags::MNT_UPDATE) {
        todo!("update root FS mounting");
    }

    // Get options.
    let system: PathBuf = opts.remove("ob:system").unwrap();
    let game: PathBuf = opts.remove("ob:game").unwrap();
    let param: Arc<Param> = opts.remove("ob:param").unwrap();

    // Create dev mount point.
    let dev = system.join("dev");

    if let Err(e) = create_dir(&dev) {
        if e.kind() != ErrorKind::AlreadyExists {
            return Err(Box::new(MountError::CreateDirectoryFailed(dev, e)));
        }
    }

    // Map root.
    let mut map: HashMap<VPathBuf, MountSource> = HashMap::new();

    map.insert(VPathBuf::new(), MountSource::Host(system.clone()));

    // Create a directory for game PFS.
    let mut pfs = system.join("mnt");

    pfs.push("sandbox");
    pfs.push("pfsmnt");

    if let Err(e) = std::fs::create_dir_all(&pfs) {
        panic!("Cannot create {}: {}.", pfs.display(), e);
    }

    // Map game PFS.
    let pfs: VPathBuf = format!("/mnt/sandbox/pfsmnt/{}-app0-patch0-union", param.title_id())
        .try_into()
        .unwrap();

    map.insert(pfs.clone(), MountSource::Host(game));

    // Create a directory for app0.
    let mut app = system.join("mnt");

    app.push("sandbox");
    app.push(format!("{}_000", param.title_id()));

    if let Err(e) = std::fs::create_dir_all(&app) {
        panic!("Cannot create {}: {}.", app.display(), e);
    }

    // Map /mnt/sandbox/{id}_000/app0 to /mnt/sandbox/pfsmnt/{id}-app0-patch0-union.
    let app: VPathBuf = format!("/mnt/sandbox/{}_000", param.title_id())
        .try_into()
        .unwrap();

    map.insert(app.join("app0").unwrap(), MountSource::Bind(pfs));

    // Open root directory.
    let root = match HostFile::root(&system) {
        Ok(v) => v,
        Err(e) => return Err(Box::new(MountError::OpenRootFailed(system, e))),
    };

    // Set mount data.
    Ok(Mount::new(
        conf,
        cred,
        super::MountSource::Driver("md0"), // TODO: Actually it is /dev/md0 but the PS4 show as md0.
        path,
        parent,
        flags | MountFlags::MNT_LOCAL,
        HostFs {
            root: Arc::new(root),
            actives: Mutex::default(),
        },
    ))
}

impl Filesystem for HostFs {
    fn root(self: Arc<Self>, mnt: &Arc<Mount>) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        let vnode = get_vnode(&self, mnt, &self.root)?;
        Ok(vnode)
    }
}

fn get_vnode(
    fs: &Arc<HostFs>,
    mnt: &Arc<Mount>,
    file: &Arc<HostFile>,
) -> Result<Arc<Vnode>, GetVnodeError> {
    // Get file ID.
    let id = match file.id() {
        Ok(v) => v,
        Err(e) => return Err(GetVnodeError::GetFileIdFailed(e)),
    };

    // Check if active.
    let mut actives = fs.actives.lock().unwrap();

    if let Some(v) = actives.get(&id).and_then(|v| v.upgrade()) {
        return Ok(v);
    }

    // Get vnode type. Beware of deadlock here.
    let ty = match file.is_directory() {
        Ok(true) => VnodeType::Directory(Arc::ptr_eq(file, &fs.root)),
        Ok(false) => VnodeType::File,
        Err(e) => return Err(GetVnodeError::GetFileTypeFailed(e)),
    };

    // Allocate a new vnode.
    let vn = Vnode::new(
        mnt,
        ty,
        "exfatfs",
        VnodeBackend::new(fs.clone(), file.clone()),
    );

    actives.insert(id, Arc::downgrade(&vn));

    Ok(vn)
}

/// Source of mount point.
#[derive(Debug)]
enum MountSource {
    Host(PathBuf),
    Bind(VPathBuf),
}

/// Represents an error when [`mount()`] fails.
#[derive(Debug, Error, Errno)]
enum MountError {
    #[error("cannot create {0}")]
    #[errno(EIO)]
    CreateDirectoryFailed(PathBuf, #[source] std::io::Error),

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
