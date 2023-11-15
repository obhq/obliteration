use crate::{fs::{VFile, Fd}, syscalls::SysErr, errno::EBADF};
use gmtx::{GroupMutex, MutexGroup};
use std::sync::Arc;

/// An implementation of `filedesc` structure.
#[derive(Debug)]
pub struct VProcFiles {
    files: GroupMutex<Vec<Option<Arc<VFile>>>>, // fd_ofiles
}

impl VProcFiles {
    pub(super) fn new(mg: &Arc<MutexGroup>) -> Self {
        Self {
            files: mg.new_member(vec![None, None, None]),
        }
    }

    /// See `finstall` on the PS4 for a reference.
    pub fn alloc(&self, file: Arc<VFile>) -> Fd {
        // TODO: Implement fdalloc.
        let mut files = self.files.write();

        for i in 3..=Fd::MAX {
            let i: usize = i.try_into().unwrap();

            if i == files.len() {
                files.push(Some(file));
            } else if files[i].is_none() {
                files[i] = Some(file);
            } else {
                continue;
            }

            return i as Fd;
        }

        // This should never happened.
        panic!("Too many files has been opened.");
    }

    /// See `fget` on the PS4 for a reference.
    pub fn get(&self, fd: Fd) -> Option<Arc<VFile>> {
        // TODO: Check what we have missed here.
        if fd < 0 {
            return None;
        }

        let fd: usize = fd.try_into().unwrap();
        let files = self.files.read();

        files.get(fd)?.clone()
    }

    pub fn free(&self, fd: Fd) -> Result<(), SysErr> {
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
