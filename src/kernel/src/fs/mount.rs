use super::{Fs, FsConfig, VPathBuf, Vnode};
use crate::ucred::Ucred;
use bitflags::bitflags;
use gmtx::{Gutex, GutexGroup, GutexReadGuard, GutexWriteGuard};
use param::Param;
use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;
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
        let v2 = m.fsconf.ty;
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
    fsconf: &'static FsConfig,                // mnt_vfc
    gen: i32,                                 // mnt_gen
    data: Option<Arc<dyn Any + Send + Sync>>, // mnt_data
    cred: Arc<Ucred>,                         // mnt_cred
    parent: Gutex<Option<Arc<Vnode>>>,        // mnt_vnodecovered
    flags: Gutex<MountFlags>,                 // mnt_flag
    stats: FsStats,                           // mnt_stat
}

impl Mount {
    /// See `vfs_mount_alloc` on the PS4 for a reference.
    pub fn new(
        parent: Option<Arc<Vnode>>,
        fsconf: &'static FsConfig,
        path: impl Into<String>,
        cred: &Arc<Ucred>,
    ) -> Self {
        let gg = GutexGroup::new();
        let owner = cred.effective_uid();
        let mount = Self {
            fsconf,
            gen: 1,
            data: None,
            cred: cred.clone(),
            parent: gg.spawn(parent),
            flags: gg.spawn(MountFlags::empty()),
            stats: FsStats {
                ty: fsconf.ty,
                id: [0; 2],
                owner,
                path: path.into(),
            },
        };

        mount
    }

    pub fn fs(&self) -> &'static FsConfig {
        self.fsconf
    }

    pub fn data(&self) -> Option<&Arc<dyn Any + Send + Sync>> {
        self.data.as_ref()
    }

    pub fn set_data(&mut self, v: Arc<dyn Any + Send + Sync>) {
        self.data = Some(v);
    }

    pub fn parent(&self) -> GutexReadGuard<Option<Arc<Vnode>>> {
        self.parent.read()
    }

    pub fn parent_mut(&self) -> GutexWriteGuard<Option<Arc<Vnode>>> {
        self.parent.write()
    }

    pub fn flags(&self) -> GutexReadGuard<MountFlags> {
        self.flags.read()
    }

    pub fn flags_mut(&self) -> GutexWriteGuard<MountFlags> {
        self.flags.write()
    }

    pub fn root(self: &Arc<Self>) -> Arc<Vnode> {
        (self.fsconf.ops.root)(self)
    }
}

bitflags! {
    /// Flags for [`Mount`].
    #[derive(Debug, Clone, Copy)]
    pub struct MountFlags: u64 {
        const MNT_RDONLY = 0x0000000000000001;
        const MNT_NOSUID = 0x0000000000000008;
        const MNTK_LOOKUP_EXCL_DOTDOT = 0x0000000000000800;
        const MNT_LOCAL = 0x0000000000001000;
        const MNT_ROOTFS = 0x0000000000004000;
        const MNT_USER = 0x0000000000008000;
        const MNT_UPDATE = 0x0000000000010000;
    }
}

pub(super) struct MountOpts(HashMap<&'static str, MountOpt>);

impl MountOpts {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, k: &'static str, v: impl Into<MountOpt>) {
        self.0.insert(k.into(), v.into());
    }

    pub fn remove(&mut self, k: &'static str) -> Option<MountOpt> {
        self.0.remove(k)
    }

    pub fn retain(&mut self, mut f: impl FnMut(&&'static str, &mut MountOpt) -> bool) {
        self.0.retain(|k, v| f(k, v));
    }
}

pub(super) enum MountOpt {
    Bool(bool),
    Int(i32),
    Str(Box<str>),
    VPathBuf(VPathBuf),
    PathBuf(PathBuf),
    Param(Arc<Param>),
}

impl MountOpt {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }
}

impl From<bool> for MountOpt {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl From<i32> for MountOpt {
    fn from(v: i32) -> Self {
        Self::Int(v)
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

impl From<VPathBuf> for MountOpt {
    fn from(v: VPathBuf) -> Self {
        Self::VPathBuf(v)
    }
}

impl From<PathBuf> for MountOpt {
    fn from(v: PathBuf) -> Self {
        Self::PathBuf(v)
    }
}

impl From<Arc<Param>> for MountOpt {
    fn from(v: Arc<Param>) -> Self {
        Self::Param(v)
    }
}

impl TryFrom<MountOpt> for bool {
    type Error = ();

    fn try_from(v: MountOpt) -> Result<Self, Self::Error> {
        match v {
            MountOpt::Bool(v) => Ok(v),
            _ => Err(()),
        }
    }
}

impl TryFrom<MountOpt> for i32 {
    type Error = ();

    fn try_from(v: MountOpt) -> Result<Self, Self::Error> {
        match v {
            MountOpt::Int(v) => Ok(v),
            _ => Err(()),
        }
    }
}

impl TryFrom<MountOpt> for Box<str> {
    type Error = ();

    fn try_from(v: MountOpt) -> Result<Self, Self::Error> {
        match v {
            MountOpt::Str(v) => Ok(v),
            _ => Err(()),
        }
    }
}

impl TryFrom<MountOpt> for VPathBuf {
    type Error = ();

    fn try_from(v: MountOpt) -> Result<Self, Self::Error> {
        match v {
            MountOpt::VPathBuf(v) => Ok(v),
            _ => Err(()),
        }
    }
}

impl TryFrom<MountOpt> for PathBuf {
    type Error = ();

    fn try_from(v: MountOpt) -> Result<Self, Self::Error> {
        match v {
            MountOpt::PathBuf(v) => Ok(v),
            _ => Err(()),
        }
    }
}

impl TryFrom<MountOpt> for Arc<Param> {
    type Error = ();

    fn try_from(v: MountOpt) -> Result<Self, Self::Error> {
        match v {
            MountOpt::Param(v) => Ok(v),
            _ => Err(()),
        }
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
