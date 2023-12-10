use super::Fs;
use crate::errno::Errno;
use crate::process::VThread;
use crate::ucred::Ucred;
use bitflags::bitflags;
use std::fmt::{Debug, Display, Formatter};

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
        self.ops.as_deref()
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
    fn write(
        &self,
        file: &VFile,
        data: &[u8],
        cred: &Ucred,
        td: &VThread,
    ) -> Result<usize, Box<dyn Errno>>;

    fn ioctl(
        &self,
        file: &VFile,
        com: IoctlCom,
        data: &mut [u8],
        cred: &Ucred,
        td: &VThread,
    ) -> Result<(), Box<dyn Errno>>;
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct IoctlCom(u32);

impl Display for IoctlCom {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:x}", self.0)
    }
}

impl IoctlCom {
    pub const IOCPARM_SHIFT: u32 = 13;
    pub const IOCPARM_MASK: u32 = (1 << Self::IOCPARM_SHIFT) - 1;
    pub const IOC_VOID: u32 = 0x20000000;
    pub const IOC_OUT: u32 = 0x40000000;
    pub const IOC_IN: u32 = 0x80000000;
    pub const IOC_INOUT: u32 = Self::IOC_IN | Self::IOC_OUT;

    pub const fn try_from_raw(com: u64) -> Option<Self> {
        let com = com as u32;

        if Self::is_invalid(com) {
            return None;
        }

        Some(Self(com))
    }

    const fn new(inout: u32, group: u8, num: u8, len: usize) -> Self {
        let len: u32 = if len > (u32::MAX) as usize {
            panic!("IOCPARM_LEN is too large");
        } else {
            len as u32
        };

        Self(inout | ((len & Self::IOCPARM_MASK) << 16) | ((group as u32) << 8) | (num as u32))
    }

    pub fn size(&self) -> usize {
        Self::iocparm_len(self.0) as usize
    }

    pub fn is_void(&self) -> bool {
        self.0 & Self::IOC_VOID != 0
    }

    pub fn is_out(&self) -> bool {
        self.0 & Self::IOC_OUT != 0
    }

    pub fn is_in(&self) -> bool {
        self.0 & Self::IOC_IN != 0
    }

    const fn is_invalid(com: u32) -> bool {
        if com & (Self::IOC_VOID | Self::IOC_IN | Self::IOC_OUT) == 0 {
            return false;
        }

        if com & (Self::IOC_IN | Self::IOC_OUT) != 0 && Self::iocparm_len(com) == 0 {
            return false;
        }

        if com & Self::IOC_VOID != 0 && Self::iocparm_len(com) != 0 && Self::iocparm_len(com) != 4 {
            return false;
        }

        true
    }

    const fn iocparm_len(com: u32) -> usize {
        ((com >> 16) & Self::IOCPARM_MASK) as usize
    }

    pub const fn io(group: u8, num: u8) -> Self {
        Self::new(Self::IOC_VOID, group, num, 0)
    }

    pub const fn iowint(group: u8, num: u8) -> Self {
        Self::new(Self::IOC_VOID, group, num, std::mem::size_of::<i32>())
    }

    pub const fn ior<T>(group: u8, num: u8) -> Self {
        Self::new(Self::IOC_OUT, group, num, std::mem::size_of::<T>())
    }

    pub const fn iow<T>(group: u8, num: u8) -> Self {
        Self::new(Self::IOC_IN, group, num, std::mem::size_of::<T>())
    }

    pub const fn iowr<T>(group: u8, num: u8) -> Self {
        Self::new(Self::IOC_INOUT, group, num, std::mem::size_of::<T>())
    }
}

bitflags! {
    /// Flags for [`VFile`].
    #[derive(Debug, Clone, Copy)]
    pub struct VFileFlags: u32 {
        const FREAD = 0x00000001;
        const FWRITE = 0x00000002;
    }
}
