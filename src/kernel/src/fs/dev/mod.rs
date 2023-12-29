pub use self::cdev::*;
use self::dirent::{Dirent, DirentFlags};
use self::vnode::VNODE_OPS;
use super::{path_contains, DirentType, FsOps, Mount, MountFlags, Vnode, VnodeType};
use crate::errno::{Errno, EEXIST, EOPNOTSUPP};
use crate::ucred::Ucred;
use bitflags::bitflags;
use std::any::Any;
use std::collections::HashMap;
use std::num::NonZeroI32;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use thiserror::Error;

mod cdev;
pub(super) mod console;
pub(super) mod deci_tty6;
pub(super) mod dipsw;
mod dirent;
pub(super) mod dmem0;
pub(super) mod dmem1;
pub(super) mod dmem2;
mod vnode;

/// See `make_dev_credv` on the PS4 for a reference.
pub fn make_dev<N: Into<String>>(
    sw: &Arc<CdevSw>,
    unit: i32,
    name: N,
    uid: i32,
    gid: i32,
    mode: u16,
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
pub fn dev_exists<N: AsRef<str>>(name: N) -> bool {
    let name = name.as_ref();

    for dev in &DEVICES.read().unwrap().list {
        if path_contains(dev.name(), name) || path_contains(name, dev.name()) {
            return true;
        }
    }

    // TODO: Implement devfs_dir_find.
    false
}

/// An implementation of `devfs_mount` structure.
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
                let d = match c.iter().find(|&c| c.dirent().name() == n) {
                    Some(c) => {
                        if c.dirent().ty() == DirentType::Link {
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
                .find(|&c| c.dirent().ty() == DirentType::Link && c.dirent().name() == name)
                .is_some()
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
                DirentFlags::empty(),
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

    /// Partial implementation of `devfs_vmkdir`. The main different is this function does not add
    /// the created directory to `parent` and does not run `devfs_rules_apply`.
    fn mkdir<N: Into<String>>(name: N, inode: i32, parent: Option<&Arc<Dirent>>) -> Arc<Dirent> {
        // Create the directory.
        let dir = Arc::new(Dirent::new(
            DirentType::Directory,
            if inode == 0 {
                INODE.fetch_add(1, Ordering::Relaxed).try_into().unwrap()
            } else {
                inode
            },
            0,
            0,
            0555,
            None,
            DirentFlags::empty(),
            name,
        ));

        // Add "." directory.
        let dot = Dirent::new(
            DirentType::Directory,
            0,
            0,
            0,
            0,
            Some(Arc::downgrade(&dir)),
            DirentFlags::DE_DOT,
            ".",
        );

        dir.children_mut().push(Arc::new(dot));

        // Add ".." directory.
        let dd = Dirent::new(
            DirentType::Directory,
            0,
            0,
            0,
            0,
            Some(Arc::downgrade(parent.unwrap_or(&dir))),
            DirentFlags::DE_DOTDOT,
            "..",
        );

        dir.children_mut().push(Arc::new(dd));
        dir
    }

    /// See `devfs_allocv` on the PS4 for a reference.
    fn alloc_vnode(mnt: &Arc<Mount>, ent: &Arc<Dirent>) -> Arc<Vnode> {
        // Get type.
        let ty = match ent.dirent().ty() {
            DirentType::Character => todo!("devfs_allocv with DT_CHR"),
            DirentType::Directory => VnodeType::Directory(ent.inode() == Self::DEVFS_ROOTINO),
            DirentType::Link => todo!("devfs_allocv with DT_LNK"),
        };

        // Create vnode.
        let vn = Arc::new(Vnode::new(mnt, ty, "devfs", &VNODE_OPS, ent.clone()));
        let mut current = ent.vnode_mut();

        if let Some(_) = current.as_ref().and_then(|v| v.upgrade()) {
            todo!("devfs_allocv with non-null vnode");
        }

        *current = Some(Arc::downgrade(&vn));
        drop(current);

        // TODO: Implement insmntque1.
        vn
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

fn mount(mount: &mut Mount, _: HashMap<String, Box<dyn Any>>) -> Result<(), Box<dyn Errno>> {
    // Check mount flags.
    let mut flags = mount.flags_mut();

    if flags.intersects(MountFlags::MNT_ROOTFS) {
        return Err(Box::new(MountError::RootFs));
    } else if flags.intersects(MountFlags::MNT_UPDATE) {
        return Err(Box::new(MountError::Update));
    }

    flags.set(MountFlags::MNT_LOCAL, true);

    drop(flags);

    // Set mount data.
    let index = DEVFS_INDEX.fetch_add(1, Ordering::Relaxed);

    mount.set_data(Arc::new(DevFs {
        index,
        root: DevFs::mkdir("", DevFs::DEVFS_ROOTINO, None),
        generation: Mutex::new(0),
    }));

    Ok(())
}

fn root(mnt: &Arc<Mount>) -> Arc<Vnode> {
    let fs = mnt.data().unwrap().downcast_ref::<DevFs>().unwrap();

    DevFs::alloc_vnode(mnt, &fs.root)
}

/// Represents an error when [`mount`] is failed.
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

pub(super) static DEVFS_OPS: FsOps = FsOps { mount, root };
static DEVFS_INDEX: AtomicUsize = AtomicUsize::new(0); // TODO: Use a proper implementation.
static INODE: AtomicU32 = AtomicU32::new(3); // TODO: Same here.
static DEVICES: RwLock<Devices> = RwLock::new(Devices {
    list: Vec::new(),
    generation: 0,
});
