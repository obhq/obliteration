use crate::errno::ENOTTY;
use crate::syscalls::{SysArg, SysErr};
use std::fmt::{Display, Formatter};

/// A wrapper type for and IOCTL command.
/// FreeBSD uses an u_long, but masks off the top 4 bytes in kern_ioctl, so we can use an u32.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoCmd(u32);

impl IoCmd {
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
        Self::iocparm_len(self.0)
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
            return true;
        }

        if com & (Self::IOC_IN | Self::IOC_OUT) != 0 && Self::iocparm_len(com) == 0 {
            return true;
        }

        if com & Self::IOC_VOID != 0 && Self::iocparm_len(com) != 0 && Self::iocparm_len(com) != 4 {
            return true;
        }

        false
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

    /// An implementation of the FreeBSD CMDGROUP macro.
    pub fn group(&self) -> u8 {
        ((self.0 >> 8) & 0xff) as u8
    }
}

impl TryFrom<SysArg> for IoCmd {
    type Error = SysErr;

    fn try_from(v: SysArg) -> Result<Self, Self::Error> {
        Self::try_from_raw(v.into()).ok_or(SysErr::Raw(ENOTTY))
    }
}

impl Display for IoCmd {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}
