use self::dirent::DirentFlags;
use super::{DirentType, FsOps, Mount, MountFlags, Vnode, VnodeType, VopVector};
use crate::errno::{Errno, EOPNOTSUPP};
use std::any::Any;
use std::collections::HashMap;
use std::num::NonZeroI32;
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use std::sync::Arc;
use thiserror::Error;

pub(super) mod console;
pub(super) mod deci_tty6;
pub(super) mod dipsw;
mod dirent;
pub(super) mod dmem0;
pub(super) mod dmem1;
pub(super) mod dmem2;

/// An implementation of `devfs_mount` structure.
pub struct DevFs {
    idx: u32,                        // dm_idx
    root: Arc<self::dirent::Dirent>, // dm_rootdir
}

impl DevFs {
    const DEVFS_ROOTINO: i32 = 2;

    /// See `devfs_vmkdir` on the PS4 for a reference.
    fn mkdir<N: Into<String>>(
        name: N,
        inode: i32,
        parent: Option<Arc<self::dirent::Dirent>>,
    ) -> Arc<self::dirent::Dirent> {
        use self::dirent::Dirent;

        // Create the directory.
        let dir = Arc::new(Dirent::new(
            DirentType::Directory,
            if inode == 0 {
                INODE.fetch_add(1, Ordering::Relaxed).try_into().unwrap()
            } else {
                inode
            },
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
            Some(Arc::downgrade(parent.as_ref().unwrap_or(&dir))),
            DirentFlags::DE_DOTDOT,
            "..",
        );

        dir.children_mut().push(Arc::new(dd));

        if let Some(p) = parent {
            // TODO: Implement devfs_rules_apply.
            p.children_mut().push(dir.clone());
        }

        dir
    }

    /// See `devfs_allocv` on the PS4 for a reference.
    fn alloc_vnode(mnt: &Arc<Mount>, ent: &Arc<self::dirent::Dirent>) -> Arc<Vnode> {
        // Get type.
        let ty = match ent.dirent().ty() {
            DirentType::Character => todo!("devfs_allocv with DT_CHR"),
            DirentType::Directory => VnodeType::Directory(ent.inode() == Self::DEVFS_ROOTINO),
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
    let idx = DEVFS.fetch_add(1, Ordering::Relaxed);

    mount.set_data(Arc::new(DevFs {
        idx: idx.try_into().unwrap(),
        root: DevFs::mkdir("", DevFs::DEVFS_ROOTINO, None),
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
static DEVFS: AtomicI32 = AtomicI32::new(0); // TODO: Use a proper implementation.
static INODE: AtomicU32 = AtomicU32::new(3); // TODO: Same here.
static VNODE_OPS: VopVector = VopVector {};
