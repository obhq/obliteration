use crate::fs::DirentType;
use bitflags::bitflags;
use gmtx::{Gutex, GutexGroup, GutexWriteGuard};
use std::sync::Arc;
use std::time::SystemTime;

/// An implementation of `devfs_dirent` structure.
pub struct Dirent {
    inode: i32,                      // de_inode
    mode: u16,                       // de_mode
    dir: Option<Arc<Self>>,          // de_dir
    children: Gutex<Vec<Arc<Self>>>, // de_dlist
    ctime: SystemTime,               // de_ctime
    atime: SystemTime,               // de_atime
    mtime: SystemTime,               // de_mtime
    flags: DirentFlags,              // de_flags
    dirent: crate::fs::Dirent,       // de_dirent
}

impl Dirent {
    pub fn new<N>(
        ty: DirentType,
        inode: i32,
        mode: u16,
        dir: Option<Arc<Self>>,
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
            mode,
            dir,
            children: gg.spawn(Vec::new()),
            ctime: now,
            atime: now,
            mtime: now,
            flags,
            dirent: crate::fs::Dirent::new(ty, name),
        }
    }

    pub fn children_mut(&self) -> GutexWriteGuard<Vec<Arc<Self>>> {
        self.children.write()
    }
}

bitflags! {
    /// Flags for [`Dirent`].
    pub struct DirentFlags: u32 {
        const DE_DOT = 0x02;
        const DE_DOTDOT = 0x04;
    }
}
