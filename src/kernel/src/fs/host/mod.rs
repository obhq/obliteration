use self::vnode::VNODE_OPS;
use super::{FsOps, Mount, MountFlags, VPath, VPathBuf, Vnode, VnodeType};
use crate::errno::Errno;
use param::Param;
use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

mod vnode;

/// Mount data for host FS.
///
/// We subtitute `exfatfs` with this because the root FS on the PS4 is exFAT. That mean we must
/// report this as `exfatfs` otherwise it might be unexpected by the PS4.
pub struct HostFs {
    map: HashMap<VPathBuf, MountSource>,
    app: Arc<VPathBuf>,
}

impl HostFs {
    pub fn app(&self) -> &Arc<VPathBuf> {
        &self.app
    }

    fn resolve(&self, path: &VPath) -> Option<FsItem> {
        let mut current = VPathBuf::new();
        let root = match self.map.get(&current).unwrap() {
            MountSource::Host(v) => v,
            MountSource::Bind(_) => unreachable!(),
        };

        // Walk on virtual path components.
        let mut directory = HostDir {
            path: root.clone(),
            vpath: VPathBuf::new(),
        };

        for component in path.components() {
            current.push(component).unwrap();

            // Check if a virtual path is a mount point.
            if let Some(mount) = self.map.get(&current) {
                let path = match mount {
                    MountSource::Host(v) => v.to_owned(),
                    MountSource::Bind(v) => match self.resolve(v)? {
                        FsItem::Directory(d) => d.path,
                        _ => unreachable!(),
                    },
                };

                directory = HostDir {
                    path,
                    vpath: VPathBuf::new(),
                };
            } else {
                // Build a real path.
                let mut path = directory.path;

                path.push(component);

                // Get file metadata.
                let meta = match std::fs::metadata(&path) {
                    Ok(v) => v,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::NotFound {
                            return None;
                        } else {
                            panic!("Cannot get the metadata of {}: {e}.", path.display());
                        }
                    }
                };

                // Check file type.
                if meta.is_file() {
                    return Some(FsItem::File(HostFile {
                        path,
                        vpath: current,
                    }));
                }

                directory = HostDir {
                    path,
                    vpath: VPathBuf::new(),
                };
            }
        }

        // If we reached here that mean the the last component is a directory.
        Some(FsItem::Directory(HostDir {
            path: directory.path,
            vpath: current,
        }))
    }
}

fn mount(mount: &mut Mount, mut opts: HashMap<String, Box<dyn Any>>) -> Result<(), Box<dyn Errno>> {
    // Check mount flags.
    let mut flags = mount.flags_mut();

    if !flags.intersects(MountFlags::MNT_ROOTFS) {
        todo!("mounting host FS on non-root");
    } else if flags.intersects(MountFlags::MNT_UPDATE) {
        todo!("update root FS mounting");
    }

    flags.set(MountFlags::MNT_LOCAL, true); // TODO: Check if this flag has been set for exfatfs.

    drop(flags);

    // Get options.
    let system = opts
        .remove("ob:system")
        .unwrap()
        .downcast::<PathBuf>()
        .unwrap();
    let game = opts
        .remove("ob:game")
        .unwrap()
        .downcast::<PathBuf>()
        .unwrap();
    let param = opts
        .remove("ob:param")
        .unwrap()
        .downcast::<Arc<Param>>()
        .unwrap();

    // Map root.
    let mut map: HashMap<VPathBuf, MountSource> = HashMap::new();

    map.insert(VPathBuf::new(), MountSource::Host(*system.clone()));

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

    map.insert(pfs.clone(), MountSource::Host(*game));

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
    mount.set_data(Arc::new(HostFs {
        map,
        app: Arc::new(app),
    }));

    Ok(())
}

fn root(mnt: &Arc<Mount>) -> Arc<Vnode> {
    Arc::new(Vnode::new(
        mnt,
        VnodeType::Directory(true),
        "exfatfs",
        &VNODE_OPS,
        Arc::new(VPathBuf::new()),
    ))
}

/// Source of mount point.
#[derive(Debug)]
enum MountSource {
    Host(PathBuf),
    Bind(VPathBuf),
}

enum FsItem {
    Directory(HostDir),
    File(HostFile),
}

pub struct HostDir {
    path: PathBuf,
    vpath: VPathBuf,
}

pub struct HostFile {
    path: PathBuf,
    vpath: VPathBuf,
}

pub(super) static HOST_OPS: FsOps = FsOps { mount, root };
