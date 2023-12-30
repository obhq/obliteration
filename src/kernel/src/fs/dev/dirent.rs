use crate::fs::{DirentType, Vnode};
use bitflags::bitflags;
use gmtx::{Gutex, GutexGroup, GutexReadGuard, GutexWriteGuard};
use std::ops::Deref;
use std::sync::{Arc, Weak};
use std::time::SystemTime;

/// An implementation of `devfs_dirent` structure.
pub struct Dirent {
    inode: i32,                        // de_inode
    uid: Gutex<i32>,                   // de_uid
    gid: Gutex<i32>,                   // de_gid
    mode: Gutex<u16>,                  // de_mode
    dir: Option<Weak<Self>>,           // de_dir
    children: Gutex<Vec<Arc<Self>>>,   // de_dlist
    ctime: SystemTime,                 // de_ctime
    atime: Gutex<SystemTime>,          // de_atime
    mtime: Gutex<SystemTime>,          // de_mtime
    vnode: Gutex<Option<Weak<Vnode>>>, // de_vnode
    flags: DirentFlags,                // de_flags
    dirent: crate::fs::Dirent,         // de_dirent
}

impl Dirent {
    pub fn new<N>(
        ty: DirentType,
        inode: i32,
        uid: i32,
        gid: i32,
        mode: u16,
        dir: Option<Weak<Self>>,
        flags: DirentFlags,
        name: N,
    ) -> Self
    where
        N: Into<String>,
    {
        let gg = GutexGroup::new();
        let now = SystemTime::now();

        Self {
            inode,
            uid: gg.spawn(uid),
            gid: gg.spawn(gid),
            mode: gg.spawn(mode),
            dir,
            children: gg.spawn(Vec::new()),
            ctime: now,
            atime: gg.spawn(now),
            mtime: gg.spawn(now),
            vnode: gg.spawn(None),
            flags,
            dirent: crate::fs::Dirent::new(ty, name),
        }
    }

    pub fn inode(&self) -> i32 {
        self.inode
    }

    pub fn uid(&self) -> GutexReadGuard<i32> {
        self.uid.read()
    }

    pub fn gid(&self) -> GutexReadGuard<i32> {
        self.gid.read()
    }

    pub fn mode(&self) -> GutexReadGuard<u16> {
        self.mode.read()
    }

    /// [`None`] represents self as a value.
    pub fn dir(&self) -> Option<&Weak<Self>> {
        self.dir.as_ref()
    }

    pub fn children_mut(&self) -> GutexWriteGuard<Vec<Arc<Self>>> {
        self.children.write()
    }

    pub fn vnode_mut(&self) -> GutexWriteGuard<Option<Weak<Vnode>>> {
        self.vnode.write()
    }

    pub fn flags(&self) -> DirentFlags {
        self.flags
    }

    pub fn dirent(&self) -> &crate::fs::Dirent {
        &self.dirent
    }

    /// See `devfs_find` on the PS4 for a reference.
    pub fn find<N: AsRef<str>>(&self, name: N, ty: Option<DirentType>) -> Option<Arc<Self>> {
        let name = name.as_ref();

        for child in self.children.read().deref() {
            // Check name.
            if child.dirent.name() != name {
                continue;
            }

            // Check type.
            if let Some(ty) = ty {
                if child.dirent.ty() != ty {
                    continue;
                }
            }

            return Some(child.clone());
        }

        None
    }
}

bitflags! {
    /// Flags for [`Dirent`].
    #[derive(Clone, Copy)]
    pub struct DirentFlags: u32 {
        const DE_DOT = 0x02;
        const DE_DOTDOT = 0x04;
    }
}
