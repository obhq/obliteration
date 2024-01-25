use super::{FsConfig, Mode, VPathBuf, Vnode};
use crate::arnd::Arnd;
use crate::ucred::{Gid, Ucred, Uid};
use bitflags::bitflags;
use macros::EnumConversions;
use param::Param;
use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Error};
use std::mem::MaybeUninit;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock, RwLockWriteGuard};
use thiserror::Error;

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
#[allow(dead_code)]
pub struct Mount {
    fs: &'static FsConfig, // mnt_vfc
    ops: &'static FsOps,
    gen: i32,                           // mnt_gen
    data: Arc<dyn Any + Send + Sync>,   // mnt_data
    cred: Arc<Ucred>,                   // mnt_cred
    parent: RwLock<Option<Arc<Vnode>>>, // mnt_vnodecovered
    flags: MountFlags,                  // mnt_flag
    stats: FsStats,                     // mnt_stat
    hashseed: u32,                      // mnt_hashseed
}

impl Mount {
    /// See `vfs_mount_alloc` on the PS4 for a reference.
    pub(super) fn new(
        fs: &'static FsConfig,
        ops: &'static FsOps,
        cred: &Arc<Ucred>,
        path: VPathBuf,
        parent: Option<Arc<Vnode>>,
        flags: MountFlags,
        data: impl Send + Sync + 'static,
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
            hashseed: {
                let mut buf = [0; 4];
                Arnd::rand_bytes(&mut buf);

                u32::from_le_bytes(buf)
            },
        }
    }

    pub(super) fn new_with_data_fn<D: Send + Sync + 'static>(
        fs: &'static FsConfig,
        ops: &'static FsOps,
        cred: &Arc<Ucred>,
        path: VPathBuf,
        parent: Option<Arc<Vnode>>,
        flags: MountFlags,
        data_fn: impl FnOnce(&Arc<Self>) -> Arc<D>,
    ) -> Arc<Self> {
        let owner = cred.effective_uid();

        let mnt = Self {
            fs,
            ops,
            gen: 1,
            data: Arc::new(unsafe { MaybeUninit::<D>::uninit().assume_init() }),
            cred: cred.clone(),
            parent: RwLock::new(parent),
            flags,
            stats: FsStats {
                ty: fs.ty,
                id: [0; 2],
                owner,
                path,
            },
            hashseed: {
                let mut buf = [0; 4];
                Arnd::rand_bytes(&mut buf);

                u32::from_le_bytes(buf)
            },
        };

        let mut mnt = Arc::new(mnt);
        let data = data_fn(&mnt);

        let data_ref = Arc::get_mut(&mut mnt).unwrap();

        data_ref.data = data;

        mnt
    }

    pub fn flags(&self) -> MountFlags {
        self.flags
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

    pub fn stats(&self) -> &FsStats {
        &self.stats
    }

    pub fn hashseed(&self) -> u32 {
        self.hashseed
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

pub struct MountOpts(HashMap<&'static str, MountOpt>);

impl MountOpts {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, k: &'static str, v: impl Into<MountOpt>) {
        self.0.insert(k, v.into());
    }

    pub fn remove<T>(&mut self, name: &'static str) -> Option<Result<T, MountOptError>>
    where
        T: TryFrom<MountOpt, Error = MountOpt>,
    {
        let opt = self.0.remove(name)?;

        let res = opt
            .try_into()
            .map_err(|opt| MountOptError::new::<T>(name, opt));

        Some(res)
    }

    pub fn retain(&mut self, mut f: impl FnMut(&str, &mut MountOpt) -> bool) {
        self.0.retain(|k, v| f(*k, v));
    }
}

#[derive(EnumConversions, Debug)]
pub enum MountOpt {
    Bool(bool),
    I32(i32),
    U64(u64),
    Usize(usize),
    Str(Box<str>),
    VPath(VPathBuf),
    Path(PathBuf),
    Param(Arc<Param>),
    Gid(Gid),
    Uid(Uid),
    Mode(Mode),
}

impl MountOpt {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }
}

impl From<&str> for MountOpt {
    fn from(v: &str) -> Self {
        Self::Str(v.into())
    }
}

impl From<String> for MountOpt {
    fn from(v: String) -> Self {
        Self::Str(v.into_boxed_str())
    }
}

#[derive(Debug, Error)]
pub struct MountOptError {
    optname: &'static str,
    expected: &'static str,
    got: MountOpt,
}

impl Display for MountOptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "mount opt \"{}\" is of wrong type: expected {}, got: {:?}",
            self.optname, self.expected, self.got
        )
    }
}

impl MountOptError {
    pub fn new<T>(optname: &'static str, got: MountOpt) -> Self {
        Self {
            optname,
            expected: std::any::type_name::<T>(),
            got,
        }
    }
}

/// An implementation of `statfs` structure.
#[derive(Debug)]
#[allow(dead_code)]
pub struct FsStats {
    ty: u32,        // f_type
    id: [u32; 2],   // f_fsid
    owner: Uid,     // f_owner
    path: VPathBuf, // f_mntonname
}

impl FsStats {
    pub fn id(&self) -> [u32; 2] {
        self.id
    }
}

static MOUNT_ID: Mutex<u16> = Mutex::new(0); // mntid_base + mntid_mtx
