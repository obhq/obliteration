use crate::errno::EBADF;
use crate::fs::{VFile, Vnode};
use crate::syscalls::SysErr;
use gmtx::{Gutex, GutexGroup, GutexReadGuard, GutexWriteGuard};
use std::sync::Arc;

/// An implementation of `filedesc` structure.
#[derive(Debug)]
pub struct FileDesc {
    files: Gutex<Vec<Option<Arc<VFile>>>>, // fd_ofiles
    cwd: Gutex<Option<Arc<Vnode>>>,        // fd_cdir
    root: Gutex<Option<Arc<Vnode>>>,       // fd_rdir
    jail: Gutex<Option<Arc<Vnode>>>,       // fd_jdir
}

impl FileDesc {
    pub(super) fn new(gg: &Arc<GutexGroup>) -> Self {
        Self {
            files: gg.spawn(vec![None, None, None]),
            cwd: gg.spawn(None),
            root: gg.spawn(None),
            jail: gg.spawn(None), // TODO: Check how the PS4 set this field.
        }
    }

    pub fn cwd(&self) -> Arc<Vnode> {
        self.cwd.read().clone().unwrap()
    }

    pub fn set_cwd(&self, v: Arc<Vnode>) {
        *self.cwd.write() = Some(v);
    }

    pub fn root(&self) -> Option<Arc<Vnode>> {
        self.root.read().clone()
    }

    pub fn root_mut(&self) -> GutexWriteGuard<'_, Option<Arc<Vnode>>> {
        self.root.write()
    }

    pub fn jail(&self) -> GutexReadGuard<Option<Arc<Vnode>>> {
        self.jail.read()
    }

    /// See `finstall` on the PS4 for a reference.
    pub fn alloc(&self, file: Arc<VFile>) -> i32 {
        // TODO: Implement fdalloc.
        let mut files = self.files.write();

        for i in 3..=i32::MAX {
            let i: usize = i.try_into().unwrap();

            if i == files.len() {
                files.push(Some(file));
            } else if files[i].is_none() {
                files[i] = Some(file);
            } else {
                continue;
            }

            return i as i32;
        }

        // This should never happened.
        panic!("Too many files has been opened.");
    }

    /// See `fget` on the PS4 for a reference.
    pub fn get(&self, fd: i32) -> Option<Arc<VFile>> {
        // TODO: Check what we have missed here.
        if fd < 0 {
            return None;
        }

        let fd: usize = fd.try_into().unwrap();
        let files = self.files.read();

        files.get(fd)?.clone()
    }

    pub fn free(&self, fd: i32) -> Result<(), SysErr> {
        if fd < 0 {
            return Err(SysErr::Raw(EBADF));
        }

        let fd: usize = fd.try_into().unwrap();

        let mut files = self.files.write();

        if let Some(file) = files.get_mut(fd) {
            *file = None;

            Ok(())
        } else {
            Err(SysErr::Raw(EBADF))
        }
    }
}
