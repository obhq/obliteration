use crate::fs::VFile;
use gmtx::{GroupMutex, MutexGroup};
use std::sync::Arc;

/// An implementation of `filedesc` structure.
#[derive(Debug)]
pub struct VProcFiles {
    files: GroupMutex<Vec<Option<Arc<VFile<'static>>>>>, // fd_ofiles
}

impl VProcFiles {
    pub(super) fn new(mg: &Arc<MutexGroup>) -> Self {
        Self {
            files: mg.new_member(vec![None, None, None]),
        }
    }

    /// See `finstall` on the PS4 for a reference.
    pub fn alloc(&self, file: Arc<VFile<'static>>) -> i32 {
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
    pub fn get(&self, fd: i32) -> Option<Arc<VFile<'static>>> {
        // TODO: Check what we have missed here.
        if fd < 0 {
            return None;
        }

        let fd: usize = fd.try_into().unwrap();
        let files = self.files.read();

        files.get(fd)?.clone()
    }
}
