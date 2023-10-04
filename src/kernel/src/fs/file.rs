use super::Fs;
use crate::errno::Errno;
use crate::process::VThread;
use crate::ucred::Ucred;
use bitflags::bitflags;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use std::sync::atomic::Ordering;

/// An implementation of `file` structure.
#[derive(Debug)]
pub struct VFile<'a> {
    fs: &'a Fs,
    ops: Option<Box<dyn VFileOps + 'a>>, // f_data + f_ops
    flags: VFileFlags,                   // f_flag
}

impl<'a> VFile<'a> {
    pub(super) fn new(fs: &'a Fs) -> Self {
        Self {
            fs,
            flags: VFileFlags::empty(),
            ops: None,
        }
    }

    pub fn ops(&self) -> Option<&dyn VFileOps> {
        self.ops.as_ref().map(|o| o.deref())
    }

    pub fn set_ops(&mut self, v: Option<Box<dyn VFileOps + 'a>>) {
        self.ops = v;
    }

    pub fn flags(&self) -> VFileFlags {
        self.flags
    }

    pub fn flags_mut(&mut self) -> &mut VFileFlags {
        &mut self.flags
    }
}

impl<'a> Drop for VFile<'a> {
    fn drop(&mut self) {
        self.fs.opens.fetch_sub(1, Ordering::Relaxed);
    }
}

impl<'a> Display for VFile<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.ops.as_ref().unwrap(), f)
    }
}

/// An implementation of `fileops` structure.
pub trait VFileOps: Debug + Send + Sync + Display {
    fn ioctl(
        &self,
        file: &VFile,
        com: u64,
        data: &[u8],
        cred: &Ucred,
        td: &VThread,
    ) -> Result<(), Box<dyn Errno>>;
}

bitflags! {
    /// Flags for [`VFile`].
    #[derive(Debug, Clone, Copy)]
    pub struct VFileFlags: u32 {
        const FREAD = 0x00000001;
        const FWRITE = 0x00000002;
    }
}
