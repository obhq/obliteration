use crate::fs::VFile;
use gmtx::{GroupMutex, MutexGroup};
use std::sync::Arc;

/// An implementation of `filedesc` structure.
#[derive(Debug)]
pub struct VProcFiles {
    files: GroupMutex<Vec<Option<VFile<'static>>>>, // fd_ofiles
}

impl VProcFiles {
    pub(super) fn new(mg: &Arc<MutexGroup>) -> Self {
        Self {
            files: mg.new_member(vec![None, None, None]),
        }
    }

    /// See `finstall` on the PS4 for a reference.
    pub fn alloc(&self, file: VFile<'static>) -> i32 {
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
}
