use crate::errno::EBADF;
use crate::fs::{VFile, Vnode};
use crate::syscalls::SysErr;
use gmtx::{Gutex, GutexGroup, GutexReadGuard, GutexWriteGuard};
use std::sync::Arc;

/// An implementation of `filedesc` structure.
#[derive(Debug)]
pub struct FileDesc {
    files: Gutex<Vec<Option<Arc<VFile>>>>, // fd_ofiles
    cwd: Arc<Vnode>,                       // fd_cdir
    root: Gutex<Option<Arc<Vnode>>>,       // fd_rdir
    jail: Arc<Vnode>,                      // fd_jdir
}

impl FileDesc {
    pub(super) fn new(gg: &Arc<GutexGroup>) -> Self {
        Self {
            files: gg.spawn(vec![None, None, None]),
            cwd: Arc::new(Vnode::new()), // TODO: Check how the PS4 set this field.
            root: gg.spawn(None),        // TODO: Same here.
            jail: Arc::new(Vnode::new()), // TODO: Same here.
        }
    }

    pub fn cwd(&self) -> &Arc<Vnode> {
        &self.cwd
    }

    pub fn root(&self) -> GutexReadGuard<'_, Option<Arc<Vnode>>> {
        self.root.read()
    }

    pub fn root_mut(&self) -> GutexWriteGuard<'_, Option<Arc<Vnode>>> {
        self.root.write()
    }

    pub fn jail(&self) -> &Arc<Vnode> {
        &self.jail
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
