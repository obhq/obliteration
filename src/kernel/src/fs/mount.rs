use super::{FsConfig, VPathBuf, Vnode};
use crate::ucred::{Ucred, Uid};
use bitflags::bitflags;
use std::any::Any;
use std::fmt::Debug;
use std::sync::{Arc, Mutex, RwLock, RwLockWriteGuard};

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

    pub fn remove(&mut self, m: &Arc<Mount>) {
        let i = self.0.iter().position(|i| Arc::ptr_eq(i, m)).unwrap();
        self.0.remove(i);
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        self.0.swap(a, b);
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
    fs: &'static FsConfig, // mnt_vfc
    ops: &'static FsOps,
    gen: i32,                           // mnt_gen
    data: Arc<dyn Any + Send + Sync>,   // mnt_data
    cred: Arc<Ucred>,                   // mnt_cred
    parent: RwLock<Option<Arc<Vnode>>>, // mnt_vnodecovered
    flags: MountFlags,                  // mnt_flag
    stats: FsStats,                     // mnt_stat
}

impl Mount {
    /// See `vfs_mount_alloc` on the PS4 for a reference.
    pub(super) fn new<D: Send + Sync + 'static>(
        fs: &'static FsConfig,
        ops: &'static FsOps,
        cred: &Arc<Ucred>,
        path: VPathBuf,
        parent: Option<Arc<Vnode>>,
        flags: MountFlags,
        data: D,
    ) -> Self {
        let owner = cred.effective_uid();

        Self {
            fs,
            ops,
            gen: 1,
            data: Arc::new(data),
            cred: cred.clone(),
            parent: RwLock::new(parent),
            flags,
            stats: FsStats {
                ty: fs.ty,
                id: [0; 2],
                owner,
                path,
            },
        }
    }

    pub fn data(&self) -> &Arc<dyn Any + Send + Sync> {
        &self.data
    }

    pub fn parent_mut(&self) -> RwLockWriteGuard<Option<Arc<Vnode>>> {
        self.parent.write().unwrap()
    }

    pub fn root(self: &Arc<Self>) -> Arc<Vnode> {
        (self.ops.root)(self)
    }
}

/// An implementation of `vfsops` structure.
///
/// Our version is a bit different from FreeBSD. We moved `vfs_mount` into `vfsconf`.
#[derive(Debug)]
pub(super) struct FsOps {
    pub root: fn(&Arc<Mount>) -> Arc<Vnode>, // vfs_root
}

bitflags! {
    /// Flags for [`Mount`].
    #[derive(Debug, Clone, Copy)]
    pub struct MountFlags: u64 {
        const MNT_RDONLY = 0x0000000000000001;
        const MNT_NOSUID = 0x0000000000000008;

        /// Mount is local (e.g. not a remote FS like NFS).
        const MNT_LOCAL = 0x0000000000001000;

        const MNT_ROOTFS = 0x0000000000004000;
        const MNT_USER = 0x0000000000008000;
        const MNT_UPDATE = 0x0000000000010000;
    }
}

/// An implementation of `statfs` structure.
#[derive(Debug)]
pub struct FsStats {
    ty: u32,        // f_type
    id: [u32; 2],   // f_fsid
    owner: Uid,     // f_owner
    path: VPathBuf, // f_mntonname
}

static MOUNT_ID: Mutex<u16> = Mutex::new(0); // mntid_base + mntid_mtx
