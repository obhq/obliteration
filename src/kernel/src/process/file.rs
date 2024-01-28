use crate::errno::{Errno, EBADF};
use crate::fs::{VFile, VFileFlags, Vnode};
use crate::syscalls::SysErr;
use gmtx::{Gutex, GutexGroup};
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

/// An implementation of `filedesc` structure.
#[derive(Debug)]
pub struct FileDesc {
    files: Gutex<Vec<Option<Arc<VFile>>>>, // fd_ofiles
    cwd: Gutex<Arc<Vnode>>,                // fd_cdir
    root: Gutex<Arc<Vnode>>,               // fd_rdir
}

impl FileDesc {
    pub(super) fn new(root: Arc<Vnode>) -> Self {
        let gg = GutexGroup::new();

        Self {
            files: gg.spawn(vec![None, None, None]),
            cwd: gg.spawn(root.clone()),
            root: gg.spawn(root),
        }
    }

    pub fn cwd(&self) -> Arc<Vnode> {
        self.cwd.read().clone()
    }

    pub fn root(&self) -> Arc<Vnode> {
        self.root.read().clone()
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

    pub fn get(&self, fd: i32) -> Result<Arc<VFile>, GetFileError> {
        todo!()
    }

    pub fn get_for_write(&self, fd: i32) -> Result<Arc<VFile>, GetFileError> {
        todo!()
    }

    pub fn get_for_read(&self, fd: i32) -> Result<Arc<VFile>, GetFileError> {
        todo!()
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

#[derive(Debug, Error)]
pub enum GetFileError {}

impl Errno for GetFileError {
    fn errno(&self) -> NonZeroI32 {
        todo!()
    }
}
