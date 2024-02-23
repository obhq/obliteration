use self::file::HostFile;
use self::vnode::VnodeBackend;
use super::{Filesystem, FsConfig, Mount, MountFlags, MountOpts, VPathBuf, Vnode, VnodeType};
use crate::errno::{Errno, EIO};
use crate::ucred::Ucred;
use gmtx::{Gutex, GutexGroup};
use macros::Errno;
use param::Param;
use std::collections::HashMap;
use std::fs::create_dir;
use std::io::ErrorKind;
use std::num::NonZeroI32;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};
use thiserror::Error;

mod file;
mod vnode;

/// Mount data for host FS.
///
/// We subtitute `exfatfs` with this because the root FS on the PS4 is exFAT. That mean we must
/// report this as `exfatfs` otherwise it might be unexpected by the PS4.
#[derive(Debug)]
pub struct HostFs {
    root: PathBuf,
    app: Arc<VPathBuf>,
    actives: Gutex<HashMap<PathBuf, Weak<Vnode>>>,
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

    // Set mount data.
    let gg = GutexGroup::new();

    Ok(Mount::new(
        conf,
        cred,
        super::MountSource::Driver("md0"), // TODO: Actually it is /dev/md0 but the PS4 show as md0.
        path,
        parent,
        flags | MountFlags::MNT_LOCAL,
        HostFs {
            root: system,
            app: Arc::new(app),
            actives: gg.spawn(HashMap::new()),
        },
    ))
}

impl Filesystem for HostFs {
    fn root(self: Arc<Self>, mnt: &Arc<Mount>) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        let vnode = get_vnode(&self, mnt, None)?;

        Ok(vnode)
    }
}

fn get_vnode(
    fs: &Arc<HostFs>,
    mnt: &Arc<Mount>,
    path: Option<&Path>,
) -> Result<Arc<Vnode>, GetVnodeError> {
    // Get target path.
    let path = match path {
        Some(v) => v,
        None => &fs.root,
    };

    // Check if active.
    let mut actives = fs.actives.write();

    if let Some(v) = actives.get(path).and_then(|v| v.upgrade()) {
        return Ok(v);
    }

    // Open the file. Beware of deadlock here.
    let file = match HostFile::open(path) {
        Ok(v) => v,
        Err(e) => return Err(GetVnodeError::OpenFileFailed(e)),
    };

    // Get vnode type.
    let ty = match file.is_directory() {
        Ok(true) => VnodeType::Directory(path == fs.root),
        Ok(false) => VnodeType::File,
        Err(e) => return Err(GetVnodeError::GetFileTypeFailed(e)),
    };

    // Allocate a new vnode.
    let vn = Vnode::new(mnt, ty, "exfatfs", VnodeBackend::new(fs.clone(), file));

    actives.insert(path.to_owned(), Arc::downgrade(&vn));
    drop(actives);

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
}
/// Represents an error when [`get_vnode()`] fails.
#[derive(Debug, Error)]
enum GetVnodeError {
    #[error("cannot open the specified file")]
    OpenFileFailed(#[source] std::io::Error),

    #[error("cannot determine file type")]
    GetFileTypeFailed(#[source] std::io::Error),
}

impl Errno for GetVnodeError {
    fn errno(&self) -> NonZeroI32 {
        todo!()
    }
}
