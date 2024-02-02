use super::Cdev;
use crate::fs::{DirentType, Mode, Vnode};
use crate::process::VThread;
use crate::ucred::{Gid, PrisonCheckError, Uid};
use gmtx::{Gutex, GutexGroup, GutexReadGuard, GutexWriteGuard};
use std::ops::Deref;
use std::sync::{Arc, Weak};
use std::time::SystemTime;

/// An implementation of `devfs_dirent` structure.
#[derive(Debug)]
pub struct Dirent {
    inode: i32,                        // de_inode
    uid: Gutex<Uid>,                   // de_uid
    gid: Gutex<Gid>,                   // de_gid
    mode: Gutex<Mode>,                 // de_mode
    dir: Option<Weak<Self>>,           // de_dir
    children: Gutex<Vec<Arc<Self>>>,   // de_dlist
    ctime: SystemTime,                 // de_ctime
    atime: Gutex<SystemTime>,          // de_atime
    mtime: Gutex<SystemTime>,          // de_mtime
    cdev: Option<Weak<Cdev>>,          // de_cdp
    vnode: Gutex<Option<Weak<Vnode>>>, // de_vnode
    dirent: crate::fs::Dirent,         // de_dirent
}

impl Dirent {
    pub fn new(
        ty: DirentType,
        inode: i32,
        uid: Uid,
        gid: Gid,
        mode: Mode,
        dir: Option<Weak<Self>>,
        cdev: Option<Weak<Cdev>>,
        name: impl Into<String>,
    ) -> Self {
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
            cdev,
            vnode: gg.spawn(None),
            dirent: crate::fs::Dirent::new(ty, name),
        }
    }

    pub fn inode(&self) -> i32 {
        self.inode
    }

    pub fn uid(&self) -> GutexReadGuard<Uid> {
        self.uid.read()
    }

    pub fn gid(&self) -> GutexReadGuard<Gid> {
        self.gid.read()
    }

    pub fn mode(&self) -> GutexReadGuard<Mode> {
        self.mode.read()
    }

    /// [`None`] represents self as a value.
    pub fn dir(&self) -> Option<&Weak<Self>> {
        self.dir.as_ref()
    }

    pub fn children_mut(&self) -> GutexWriteGuard<Vec<Arc<Self>>> {
        self.children.write()
    }

    pub fn cdev(&self) -> Option<&Weak<Cdev>> {
        self.cdev.as_ref()
    }

    pub fn vnode_mut(&self) -> GutexWriteGuard<Option<Weak<Vnode>>> {
        self.vnode.write()
    }

    /// See `devfs_find` on the PS4 for a reference.
    pub fn find(&self, name: impl AsRef<str>, ty: Option<DirentType>) -> Option<Arc<Self>> {
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

    /// See `devfs_parent_dirent` on the PS4 for a reference.
    pub fn parent(&self) -> Option<Arc<Self>> {
        let parent = if !self.is_directory() {
            self.dir.as_ref().unwrap().clone()
        } else if matches!(self.name(), "." | "..") {
            return None;
        } else {
            // Get de_dir from "..".
            let children = self.children.read();
            let dotdot = &children[1];

            dotdot.dir.as_ref().unwrap().clone()
        };

        parent.upgrade()
    }

    /// See `devfs_prison_check` on the PS4 for a reference.
    pub fn prison_check(&self, td: &VThread) -> Result<(), PrisonCheckError> {
        let Some(dev) = self.cdev().and_then(|dev| dev.upgrade()) else {
            return Ok(());
        };

        let Err(e) = td.cred().can_access(dev.cred().unwrap()) else {
            return Ok(());
        };

        todo!()
    }
}

impl Deref for Dirent {
    type Target = crate::fs::Dirent;

    fn deref(&self) -> &Self::Target {
        &self.dirent
    }
}
