use crate::errno::{Errno, EBADF};
use crate::fs::{VFile, VFileFlags, VFileType, Vnode};
use crate::kqueue::KernelQueue;
use gmtx::{Gutex, GutexGroup};
use macros::Errno;
use std::collections::VecDeque;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

/// An implementation of `filedesc` structure.
#[derive(Debug)]
pub struct FileDesc {
    files: Gutex<Vec<Option<Arc<VFile>>>>,          // fd_ofiles
    cwd: Gutex<Arc<Vnode>>,                         // fd_cdir
    root: Gutex<Arc<Vnode>>,                        // fd_rdir
    kqueue_list: Gutex<VecDeque<Arc<KernelQueue>>>, // fd_kqlist
}

impl FileDesc {
    pub(super) fn new(root: Arc<Vnode>) -> Arc<Self> {
        let gg = GutexGroup::new();

        Arc::new(Self {
            files: gg.spawn(vec![None, None, None]),
            cwd: gg.spawn(root.clone()),
            root: gg.spawn(root),
            kqueue_list: gg.spawn(VecDeque::new()),
        })
    }

    pub fn cwd(&self) -> Arc<Vnode> {
        self.cwd.read().clone()
    }

    pub fn root(&self) -> Arc<Vnode> {
        self.root.read().clone()
    }

    pub fn insert_kqueue(&self, kq: Arc<KernelQueue>) {
        self.kqueue_list.write().push_front(kq);
    }

    #[allow(unused_variables)] // TODO: remove when implementing; add budget argument
    pub fn alloc_with_budget<E: Errno>(
        &self,
        constructor: impl FnOnce(i32) -> Result<VFileType, E>,
        flags: VFileFlags,
    ) -> Result<i32, FileAllocError<E>> {
        todo!()
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
        let fd: usize = fd.try_into().ok()?;

        let files = self.files.read();

        files.get(fd)?.clone()
    }

    pub fn free(&self, fd: i32) -> Result<(), FreeError> {
        let fd: usize = fd.try_into().map_err(|_| FreeError::NegativeFd)?;

        let mut files = self.files.write();

        if let Some(file) = files.get_mut(fd) {
            *file = None;

            Ok(())
        } else {
            Err(FreeError::NoFile)
        }
    }
}

#[derive(Debug, Error)]
pub enum FileAllocError<E: Errno> {
    #[error(transparent)]
    Inner(E),
}

impl<E: Errno> Errno for FileAllocError<E> {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::Inner(e) => e.errno(),
        }
    }
}

#[derive(Debug, Error, Errno)]
pub enum FreeError {
    #[error("negative file descriptor provided")]
    #[errno(EBADF)]
    NegativeFd,

    #[error("no file associated with file descriptor")]
    #[errno(EBADF)]
    NoFile,
}
