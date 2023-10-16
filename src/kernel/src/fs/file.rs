use super::Fs;
use crate::errno::Errno;
use crate::process::VThread;
use crate::ucred::Ucred;
use bitflags::bitflags;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// An implementation of `file` structure.
#[derive(Debug)]
pub struct VFile {
    fs: Arc<Fs>,
    ops: Option<Box<dyn VFileOps>>, // f_data + f_ops
    flags: VFileFlags,              // f_flag
}

impl VFile {
    pub(super) fn new(fs: &Arc<Fs>) -> Self {
        Self {
            fs: fs.clone(),
            flags: VFileFlags::empty(),
            ops: None,
        }
    }

    pub fn ops(&self) -> Option<&dyn VFileOps> {
        self.ops.as_ref().map(|o| o.deref())
    }

    pub fn set_ops(&mut self, v: Option<Box<dyn VFileOps>>) {
        self.ops = v;
    }

    pub fn flags(&self) -> VFileFlags {
        self.flags
    }

    pub fn flags_mut(&mut self) -> &mut VFileFlags {
        &mut self.flags
    }
}

impl Drop for VFile {
    fn drop(&mut self) {
        self.fs.opens.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Display for VFile {
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
