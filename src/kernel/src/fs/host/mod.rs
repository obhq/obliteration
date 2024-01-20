use self::file::HostFile;
use self::vnode::VNODE_OPS;
use super::{FsConfig, FsOps, Mount, MountFlags, MountOpts, VPathBuf, Vnode, VnodeType};
use crate::errno::{Errno, EIO};
use crate::ucred::Ucred;
use gmtx::{Gutex, GutexGroup};
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
pub struct HostFs {
    root: PathBuf,
    app: Arc<VPathBuf>,
    actives: Gutex<HashMap<PathBuf, Weak<Vnode>>>,
}

impl HostFs {
    pub fn app(&self) -> &Arc<VPathBuf> {
        &self.app
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
    // Check mount flags.
    if !flags.intersects(MountFlags::MNT_ROOTFS) {
        todo!("mounting host FS on non-root");
    } else if flags.intersects(MountFlags::MNT_UPDATE) {
        todo!("update root FS mounting");
    }

    // Get options.
    let system: PathBuf = opts.remove("ob:system").unwrap().try_into().unwrap();
    let game: PathBuf = opts.remove("ob:game").unwrap().try_into().unwrap();
    let param: Arc<Param> = opts.remove("ob:param").unwrap().try_into().unwrap();

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
        &HOST_OPS,
        cred,
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

fn root(mnt: &Arc<Mount>) -> Arc<Vnode> {
    get_vnode(mnt, None).unwrap()
}

fn get_vnode(mnt: &Arc<Mount>, path: Option<&Path>) -> Result<Arc<Vnode>, GetVnodeError> {
    // Get target path.
    let fs = mnt.data().downcast_ref::<HostFs>().unwrap();
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
        Ok(false) => todo!(),
        Err(e) => return Err(GetVnodeError::GetFileTypeFailed(e)),
    };

    // Allocate a new vnode.
    let vn = Arc::new(Vnode::new(mnt, ty, "exfatfs", &VNODE_OPS, Arc::new(file)));

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

/// Represents an error when [`mount()`] was failed.
#[derive(Debug, Error)]
enum MountError {
    #[error("cannot create {0}")]
    CreateDirectoryFailed(PathBuf, #[source] std::io::Error),
}

impl Errno for MountError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::CreateDirectoryFailed(_, _) => EIO,
        }
    }
}

/// Represents an error when [`get_vnode()`] was failed.
#[derive(Debug, Error)]
enum GetVnodeError {
    #[error("cannot open the specified file")]
    OpenFileFailed(#[source] std::io::Error),

    #[error("cannot determine file type")]
    GetFileTypeFailed(#[source] std::io::Error),
}

static HOST_OPS: FsOps = FsOps { root };
