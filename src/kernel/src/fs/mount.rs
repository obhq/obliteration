use super::{FsConfig, Vnode};
use crate::ucred::Ucred;
use bitflags::bitflags;
use gmtx::{Gutex, GutexGroup, GutexWriteGuard};
use std::any::Any;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// An implementation of `mount` structure on the PS4.
#[derive(Debug)]
pub struct Mount {
    fs: &'static FsConfig,                    // mnt_vfc
    gen: i32,                                 // mnt_gen
    data: Option<Arc<dyn Any + Send + Sync>>, // mnt_data
    cred: Ucred,                              // mnt_cred
    parent: Gutex<Option<Arc<Vnode>>>,        // mnt_vnodecovered
    actives: Vec<Arc<Vnode>>,                 // mnt_activevnodelist
    flags: Gutex<MountFlags>,                 // mnt_flag
    stats: FsStats,                           // mnt_stat
}

impl Mount {
    /// See `vfs_mount_alloc` on the PS4 for a reference.
    pub fn new<P>(parent: Option<Arc<Vnode>>, fs: &'static FsConfig, path: P, cred: Ucred) -> Self
    where
        P: Into<String>,
    {
        let gg = GutexGroup::new("mount");
        let owner = cred.effective_uid();
        let mount = Self {
            fs,
            gen: 1,
            data: None,
            cred,
            parent: gg.spawn(parent),
            actives: Vec::new(),
            flags: gg.spawn(MountFlags::empty()),
            stats: FsStats {
                ty: fs.ty,
                owner,
                path: path.into(),
            },
        };

        fs.refcount.fetch_add(1, Ordering::Relaxed);

        mount
    }

    pub fn fs(&self) -> &'static FsConfig {
        self.fs
    }

    pub fn data(&self) -> Option<&Arc<dyn Any + Send + Sync>> {
        self.data.as_ref()
    }

    pub fn set_data(&mut self, v: Arc<dyn Any + Send + Sync>) {
        self.data = Some(v);
    }

    pub fn parent_mut(&self) -> GutexWriteGuard<Option<Arc<Vnode>>> {
        self.parent.write()
    }

    pub fn flags_mut(&self) -> GutexWriteGuard<MountFlags> {
        self.flags.write()
    }
}

bitflags! {
    /// Flags for [`Mount`].
    #[derive(Debug, Clone, Copy)]
    pub struct MountFlags: u64 {
        const MNT_ROOTFS = 0x0000000000004000;
        const MNT_UPDATE = 0x0000000000010000;
    }
}

/// An implementation of `statfs` structure.
#[derive(Debug)]
pub struct FsStats {
    ty: i32,      // f_type
    owner: i32,   // f_owner
    path: String, // f_mntonname
}
