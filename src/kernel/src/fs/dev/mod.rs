pub use self::cdev::*;
use self::dirent::Dirent;
use self::vnode::VnodeBackend;
use super::{
    path_contains, DirentType, Filesystem, FsConfig, Mode, Mount, MountFlags, MountOpts, VPathBuf,
    Vnode, VnodeType,
};
use crate::errno::{Errno, EEXIST, ENOENT, EOPNOTSUPP};
use crate::ucred::{Gid, Ucred, Uid};
use bitflags::bitflags;
use std::num::NonZeroI32;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use thiserror::Error;

mod cdev;
mod dirent;
mod vnode;

/// See `make_dev_credv` on the PS4 for a reference.
pub fn make_dev(
    sw: &Arc<CdevSw>,
    unit: i32,
    name: impl Into<String>,
    uid: Uid,
    gid: Gid,
    mode: Mode,
    cred: Option<Arc<Ucred>>,
    flags: MakeDev,
) -> Result<Arc<Cdev>, MakeDevError> {
    if sw.flags().intersects(DriverFlags::D_NEEDMINOR) {
        todo!("make_dev_credv with D_NEEDMINOR");
    }

    // TODO: Implement prep_devname.
    let name = name.into();

    if dev_exists(&name) {
        return Err(MakeDevError::AlreadyExist(name));
    }

    // Get device flags.
    let mut df = DeviceFlags::empty();

    if flags.intersects(MakeDev::MAKEDEV_ETERNAL) {
        df |= DeviceFlags::SI_ETERNAL;
    }

    // Create cdev.
    let dev = Arc::new(Cdev::new(
        sw,
        unit,
        name,
        uid,
        gid,
        mode,
        cred,
        df,
        INODE.fetch_add(1, Ordering::Relaxed).try_into().unwrap(),
    ));

    DEVICES.write().unwrap().push(dev.clone());

    // TODO: Implement the remaining logic from the PS4.
    Ok(dev)
}

/// See `devfs_dev_exists` on the PS4 for a reference.
pub fn dev_exists(name: impl AsRef<str>) -> bool {
    let name = name.as_ref();

    for dev in &DEVICES.read().unwrap().list {
        if path_contains(dev.name(), name) || path_contains(name, dev.name()) {
            return true;
        }
    }

    // TODO: Implement devfs_dir_find.
    false
}

/// See `devfs_allocv` on the PS4 for a reference.
fn alloc_vnode(
    fs: Arc<DevFs>,
    mnt: &Arc<Mount>,
    ent: Arc<Dirent>,
) -> Result<Arc<Vnode>, AllocVnodeError> {
    // Check for active vnode.
    let mut current = ent.vnode_mut();

    if let Some(v) = current.as_ref().and_then(|v| v.upgrade()) {
        return Ok(v);
    }

    // Create vnode. Beware of deadlock because we are currently holding on dirent lock.
    let tag = "devfs";
    let backend = VnodeBackend::new(fs, ent.clone());
    let vn = match ent.ty() {
        DirentType::Character => {
            let dev = ent
                .cdev()
                .unwrap()
                .upgrade()
                .ok_or(AllocVnodeError::DeviceGone)?;
            let vn = Vnode::new(mnt, VnodeType::Character, tag, backend);

            *vn.item_mut() = Some(dev);
            vn
        }
        DirentType::Directory => Vnode::new(
            mnt,
            VnodeType::Directory(ent.inode() == DevFs::DEVFS_ROOTINO),
            tag,
            backend,
        ),
        DirentType::Link => todo!("devfs_allocv with DT_LNK"),
    };

    // Set current vnode.

    *current = Some(Arc::downgrade(&vn));
    drop(current);

    // TODO: Implement insmntque1.
    Ok(vn)
}

/// An implementation of `devfs_mount` structure.
#[derive(Debug)]
pub struct DevFs {
    index: usize,           // dm_idx
    root: Arc<Dirent>,      // dm_rootdir
    generation: Mutex<u32>, // dm_generation
}

impl DevFs {
    const DEVFS_ROOTINO: i32 = 2;

    /// See `devfs_populate` on the PS4 for a reference.
    fn populate(&self) {
        // Check if our data already latest.
        let mut gen = self.generation.lock().unwrap();
        let devices = DEVICES.read().unwrap();

        if *gen == devices.generation {
            return;
        }

        // Populate our data.
        for dev in &devices.list {
            // Check if we already populated this device.
            let dirents = dev.dirents();

            if let Some(dirent) = dirents.get(self.index).and_then(|e| e.as_ref()) {
                // If there is a strong reference that mean it is our dirent.
                if dirent.strong_count() != 0 {
                    continue;
                }
            }

            drop(dirents);

            // Create directories along the path.
            let mut dir = self.root.clone();
            let mut name = dev.name();

            while let Some(i) = name.find('/') {
                // Check if already exists.
                let n = &name[..i];
                let mut c = dir.children_mut();
                let d = match c.iter().find(|&c| c.name() == n) {
                    Some(c) => {
                        if c.ty() == DirentType::Link {
                            todo!("devfs_populate with DT_LNK children");
                        }

                        // Not sure why FreeBSD does not check if a directory?
                        c.clone()
                    }
                    None => {
                        // TODO: Implement devfs_rules_apply.
                        let d = Self::mkdir(n, 0, Some(&dir));
                        c.push(d.clone());
                        d
                    }
                };

                drop(c);

                // Move to next component.
                dir = d;
                name = &name[(i + 1)..];
            }

            // Check if a link.
            let mut children = dir.children_mut();

            if children
                .iter()
                .any(|c| c.ty() == DirentType::Link && c.name() == name)
            {
                todo!("devfs_populate with DT_LNK children");
            }

            // Check if alias.
            let (ty, uid, gid, mode) = if dev.flags().intersects(DeviceFlags::SI_ALIAS) {
                todo!("devfs_populate with SI_ALIAS");
            } else {
                (DirentType::Character, dev.uid(), dev.gid(), dev.mode())
            };

            // Create a new entry.
            let dirent = Arc::new(Dirent::new(
                ty,
                dev.inode(),
                uid,
                gid,
                mode,
                Some(Arc::downgrade(&dir)),
                Some(Arc::downgrade(dev)),
                name,
            ));

            children.push(dirent.clone());
            drop(children);

            // TODO: Implement devfs_rules_apply.
            let mut dirents = dev.dirents_mut();

            if self.index >= dirents.len() {
                dirents.resize(self.index + 1, None);
            }

            dirents[self.index] = Some(Arc::downgrade(&dirent));
        }

        *gen = devices.generation;
    }

    /// Partial implementation of `devfs_vmkdir`. The main difference is this function does not add
    /// the created directory to `parent` and does not run `devfs_rules_apply`.
    fn mkdir(name: impl Into<String>, inode: i32, parent: Option<&Arc<Dirent>>) -> Arc<Dirent> {
        // Create the directory.
        let dir = Arc::new(Dirent::new(
            DirentType::Directory,
            if inode == 0 {
                INODE.fetch_add(1, Ordering::Relaxed).try_into().unwrap()
            } else {
                inode
            },
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o555).unwrap(),
            None,
            None,
            name,
        ));

        // Add "." directory.
        let dot = Dirent::new(
            DirentType::Directory,
            0,
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0).unwrap(),
            Some(Arc::downgrade(&dir)),
            None,
            ".",
        );

        dir.children_mut().push(Arc::new(dot));

        // Add ".." directory.
        let dd = Dirent::new(
            DirentType::Directory,
            0,
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0).unwrap(),
            Some(Arc::downgrade(parent.unwrap_or(&dir))),
            None,
            "..",
        );

        dir.children_mut().push(Arc::new(dd));
        dir
    }
}

bitflags! {
    /// Flags for [`make_dev()`].
    #[derive(Clone, Copy)]
    pub struct MakeDev: u32 {
        const MAKEDEV_ETERNAL = 0x10;
    }
}

/// List of devices in the system.
struct Devices {
    list: Vec<Arc<Cdev>>, // cdevp_list
    generation: u32,      // devfs_generation
}

impl Devices {
    fn push(&mut self, d: Arc<Cdev>) {
        self.list.push(d);
        self.generation += 1;
    }
}

/// Represents an error when [`make_dev()`] is failed.
#[derive(Debug, Error)]
pub enum MakeDevError {
    #[error("the device with the same name already exist")]
    AlreadyExist(String),
}

impl Errno for MakeDevError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::AlreadyExist(_) => EEXIST,
        }
    }
}

pub fn mount(
    conf: &'static FsConfig,
    cred: &Arc<Ucred>,
    path: VPathBuf,
    parent: Option<Arc<Vnode>>,
    _: MountOpts,
    flags: MountFlags,
) -> Result<Mount, Box<dyn Errno>> {
    // Check mount flags.
    if flags.intersects(MountFlags::MNT_ROOTFS) {
        return Err(Box::new(MountError::RootFs));
    } else if flags.intersects(MountFlags::MNT_UPDATE) {
        return Err(Box::new(MountError::Update));
    }

    // Set mount data.
    let index = DEVFS_INDEX.fetch_add(1, Ordering::Relaxed);

    Ok(Mount::new(
        conf,
        cred,
        path,
        parent,
        flags | MountFlags::MNT_LOCAL,
        DevFs {
            index,
            root: DevFs::mkdir("", DevFs::DEVFS_ROOTINO, None),
            generation: Mutex::new(0),
        },
    ))
}

impl Filesystem for DevFs {
    fn root(self: Arc<Self>, mnt: &Arc<Mount>) -> Arc<Vnode> {
        let ent = self.root.clone();

        alloc_vnode(self, mnt, ent).unwrap()
    }
}

/// Represents an error when [`mount()`] is failed.
#[derive(Debug, Error)]
enum MountError {
    #[error("mounting as root FS is not supported")]
    RootFs,

    #[error("update mounting is not supported")]
    Update,
}

impl Errno for MountError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::RootFs | Self::Update => EOPNOTSUPP,
        }
    }
}

/// Represents an error when [`alloc_vnode()`] is failed.
#[derive(Debug, Error)]
enum AllocVnodeError {
    #[error("the device already gone")]
    DeviceGone,
}

impl Errno for AllocVnodeError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::DeviceGone => ENOENT,
        }
    }
}

static DEVFS_INDEX: AtomicUsize = AtomicUsize::new(0); // TODO: Use a proper implementation.
static INODE: AtomicU32 = AtomicU32::new(3); // TODO: Same here.
static DEVICES: RwLock<Devices> = RwLock::new(Devices {
    list: Vec::new(),
    generation: 0,
});
