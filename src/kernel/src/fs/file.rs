use super::Fs;
use std::fmt::Debug;
use std::sync::atomic::Ordering;

/// An implementation of `file` structure.
#[derive(Debug)]
pub struct VFile<'a> {
    fs: &'a Fs,
    ops: Option<Box<dyn VFileOps + 'a>>, // f_data + f_ops
}

impl<'a> VFile<'a> {
    pub(super) fn new(fs: &'a Fs) -> Self {
        Self { fs, ops: None }
    }

    pub fn set_ops(&mut self, v: Option<Box<dyn VFileOps + 'a>>) {
        self.ops = v;
    }
}

impl<'a> Drop for VFile<'a> {
    fn drop(&mut self) {
        self.fs.opens.fetch_sub(1, Ordering::Relaxed);
    }
}

/// An implementation of `fileops` structure.
pub trait VFileOps: Debug + Send + Sync {}
