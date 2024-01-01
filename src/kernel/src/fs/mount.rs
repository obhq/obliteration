use super::{FsConfig, Vnode};
use crate::ucred::Ucred;
use bitflags::bitflags;
use gmtx::{Gutex, GutexGroup, GutexWriteGuard};
use std::any::Any;
use std::sync::{Arc, Mutex};

/// A collection of [`Mount`].
#[derive(Debug)]
pub(super) struct Mounts(Vec<Arc<Mount>>);

impl Mounts {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, mut m: Mount) -> Arc<Mount> {
        self.set_id(&mut m);

        let m = Arc::new(m);
        self.0.push(m.clone());
        m
    }

    pub fn remove(&mut self, i: usize) -> Arc<Mount> {
        self.0.remove(i)
    }

    pub fn root(&self) -> &Arc<Mount> {
        self.0.first().unwrap()
    }

    /// See `vfs_getnewfsid` on the PS4 for a reference.
    fn set_id(&self, m: &mut Mount) {
        let mut base = MOUNT_ID.lock().unwrap();
        let v2 = m.fs.ty;
        let mut v1 = ((*base as u32) << 8) | (*base as u32) | ((v2 << 24) | 0xff00);

        loop {
            *base = base.wrapping_add(1);

            if self
                .0
                .iter()
                .find(|&m| m.stats.id[0] == v1 && m.stats.id[1] == v2)
                .is_none()
            {
                m.stats.id[0] = v1;
                m.stats.id[1] = v2;
                return;
            }

            v1 = ((v2 << 24) | 0xff00) | (*base as u32) | ((*base as u32) << 8);
        }
    }
}

/// An implementation of `mount` structure on the PS4.
#[derive(Debug)]
pub struct Mount {
    fs: &'static FsConfig,                    // mnt_vfc
    gen: i32,                                 // mnt_gen
    data: Option<Arc<dyn Any + Send + Sync>>, // mnt_data
    cred: Arc<Ucred>,                         // mnt_cred
    parent: Gutex<Option<Arc<Vnode>>>,        // mnt_vnodecovered
    flags: Gutex<MountFlags>,                 // mnt_flag
    stats: FsStats,                           // mnt_stat
}

impl Mount {
    /// See `vfs_mount_alloc` on the PS4 for a reference.
    pub fn new<P>(
        parent: Option<Arc<Vnode>>,
        fs: &'static FsConfig,
        path: P,
        cred: &Arc<Ucred>,
    ) -> Self
    where
        P: Into<String>,
    {
        let gg = GutexGroup::new();
        let owner = cred.effective_uid();
        let mount = Self {
            fs,
            gen: 1,
            data: None,
            cred: cred.clone(),
            parent: gg.spawn(parent),
            flags: gg.spawn(MountFlags::empty()),
            stats: FsStats {
                ty: fs.ty,
                id: [0; 2],
                owner,
                path: path.into(),
            },
        };

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
        const MNT_RDONLY = 0x0000000000000001;
        const MNT_NOSUID = 0x0000000000000008;
        const MNT_LOCAL = 0x0000000000001000;
        const MNT_ROOTFS = 0x0000000000004000;
        const MNT_USER = 0x0000000000008000;
        const MNT_UPDATE = 0x0000000000010000;
    }
}

/// An implementation of `statfs` structure.
#[derive(Debug)]
pub struct FsStats {
    ty: u32,      // f_type
    id: [u32; 2], // f_fsid
    owner: i32,   // f_owner
    path: String, // f_mntonname
}

static MOUNT_ID: Mutex<u16> = Mutex::new(0); // mntid_base + mntid_mtx
