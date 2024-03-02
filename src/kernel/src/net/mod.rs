use crate::budget::BudgetType;
use crate::errno::{Errno, EFAULT, EINVAL};
use crate::fs::{IoVec, VFileFlags, VFileType};
use crate::{
    process::VThread,
    syscalls::{SysErr, SysIn, SysOut, Syscalls},
};
use bitflags::bitflags;
use core::fmt;
use macros::Errno;
use std::num::NonZeroI32;
use std::{
    fmt::{Display, Formatter},
    sync::Arc,
};
use thiserror::Error;

pub use self::socket::*;

mod socket;

pub struct NetManager {}

impl NetManager {
    pub fn new(sys: &mut Syscalls) -> Arc<Self> {
        let net = Arc::new(Self {});

        sys.register(27, &net, Self::sys_recvmsg);
        sys.register(28, &net, Self::sys_sendmsg);
        sys.register(29, &net, Self::sys_recvfrom);
        sys.register(97, &net, Self::sys_socket);
        sys.register(105, &net, Self::sys_setsockopt);
        sys.register(113, &net, Self::sys_socketex);
        sys.register(105, &net, Self::sys_getsockopt);
        sys.register(133, &net, Self::sys_sendto);

        net
    }

    #[allow(unused_variables)] // TODO: Remove this when implementing
    fn sys_recvmsg(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let fd: i32 = i.args[0].try_into().unwrap();
        let msg: *mut MsgHdr = i.args[1].into();
        let flags = {
            let flags = TryInto::<u32>::try_into(i.args[2]).unwrap();
            MessageFlags::from_bits_retain(flags)
        };

        todo!()
    }

    fn sys_sendmsg(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let fd: i32 = i.args[0].try_into().unwrap();
        let msg: *const MsgHdr = i.args[1].into();
        let flags = {
            let flags = TryInto::<u32>::try_into(i.args[2]).unwrap();
            MessageFlags::from_bits_retain(flags)
        };

        let sent = self.sendit(fd, unsafe { &*msg }, flags, td)?;

        Ok(sent.into())
    }

    #[allow(unused_variables)] // TODO: Remove this when implementing
    fn sys_recvfrom(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let fd: i32 = i.args[0].try_into().unwrap();
        let buf: *mut u8 = i.args[1].into();
        let buflen: usize = i.args[2].into();
        let flags = {
            let flags = TryInto::<u32>::try_into(i.args[3]).unwrap();
            MessageFlags::from_bits_retain(flags)
        };

        todo!()
    }

    fn sys_setsockopt(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let fd: i32 = i.args[0].try_into().unwrap();
        let level: i32 = i.args[1].try_into().unwrap();
        let name: i32 = i.args[2].try_into().unwrap();
        let value: *const u8 = i.args[3].into();
        let len: i32 = i.args[4].try_into().unwrap();

        self.setsockopt(fd, level, name, value, len, td)?;

        Ok(SysOut::ZERO)
    }

    fn sys_socket(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let domain: i32 = i.args[0].try_into().unwrap();
        let ty: i32 = i.args[1].try_into().unwrap();
        let proto: Option<NonZeroI32> = i.args[2].try_into().unwrap();

        let budget = if domain == 1 {
            BudgetType::FdIpcSocket
        } else {
            BudgetType::FdSocket
        };

        let fd = td.proc().files().alloc_with_budget::<SocketCreateError>(
            |_| {
                let so = Socket::new(domain, ty, proto, td.cred(), td, None)?;

                let ty = if domain == 1 {
                    VFileType::IpcSocket(so)
                } else {
                    VFileType::Socket(so)
                };

                Ok(ty)
            },
            VFileFlags::WRITE | VFileFlags::READ,
            budget,
        )?;

        Ok(fd.into())
    }

    fn sys_getsockopt(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let fd: i32 = i.args[0].try_into().unwrap();
        let level: i32 = i.args[1].try_into().unwrap();
        let name: i32 = i.args[2].try_into().unwrap();
        let value: *mut u8 = i.args[3].into();
        let len: *mut i32 = i.args[4].into();

        self.getsockopt(fd, level, name, value, len, td)?;

        Ok(SysOut::ZERO)
    }

    fn sys_socketex(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let name = unsafe { i.args[0].to_str(32)? };
        let domain: i32 = i.args[1].try_into().unwrap();
        let ty: i32 = i.args[2].try_into().unwrap();
        let proto: Option<NonZeroI32> = i.args[3].try_into().unwrap();

        let budget = if domain == 1 {
            BudgetType::FdIpcSocket
        } else {
            BudgetType::FdSocket
        };

        let fd = td.proc().files().alloc_with_budget::<SocketCreateError>(
            |_| {
                let so = Socket::new(domain, ty, proto, td.cred(), td, name)?;

                let ty = if domain == 1 {
                    VFileType::IpcSocket(so)
                } else {
                    VFileType::Socket(so)
                };

                Ok(ty)
            },
            VFileFlags::WRITE | VFileFlags::READ,
            budget,
        )?;

        Ok(fd.into())
    }

    fn sys_sendto(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let fd: i32 = i.args[0].try_into().unwrap();
        let buf: *const u8 = i.args[1].into();
        let buflen: usize = i.args[2].into();
        let flags = {
            let flags = TryInto::<u32>::try_into(i.args[3]).unwrap();
            MessageFlags::from_bits_retain(flags)
        };
        let to: *const u8 = i.args[4].into();
        let tolen: u32 = i.args[5].try_into().unwrap();

        let ref iovec = unsafe { IoVec::from_raw_parts(buf, buflen) };

        let msg = MsgHdr {
            name: to,
            len: tolen,
            iovec: iovec as *const IoVec,
            iovec_len: 1,
            control: core::ptr::null(),
            control_len: 0,
            flags: 0,
        };

        let sent = self.sendit(fd, &msg, flags, td)?;

        Ok(sent.into())
    }

    /// See `kern_setsockopt` on the PS4 for a reference.
    #[allow(unused_variables)] // TODO: Remove this when implementing
    fn setsockopt(
        &self,
        fd: i32,
        level: i32,
        name: i32,
        value: *const u8,
        len: i32,
        td: &VThread,
    ) -> Result<(), SetOptError> {
        if value.is_null() || len == 0 {
            return Err(SetOptError::InvalidValue);
        }

        if len < 0 {
            return Err(SetOptError::InvalidLength);
        }

        todo!()
    }

    /// See `kern_setsockopt` on the PS4 for a reference.
    #[allow(unused_variables)] // TODO: Remove this when implementing
    fn getsockopt(
        &self,
        fd: i32,
        level: i32,
        name: i32,
        value: *mut u8,
        len: *mut i32,
        td: &VThread,
    ) -> Result<(), GetOptError> {
        if value.is_null() {
            unsafe {
                *len = 0;
            }
        }

        if unsafe { *len } < 0 {
            return Err(GetOptError::InvalidValue);
        }

        todo!()
    }

    /// See `kern_sendit` on the PS4 for a reference.
    #[allow(unused_variables)] // TODO: Remove this when implementing
    fn sendit(
        &self,
        fd: i32,
        msg: &MsgHdr,
        flags: MessageFlags,
        td: &VThread,
    ) -> Result<usize, SendItError> {
        todo!()
    }
}

bitflags! {
    #[repr(C)]
    pub struct MessageFlags: u32 {}
}

#[repr(C)]
struct MsgHdr {
    name: *const u8,
    len: u32,
    iovec: *const IoVec,
    iovec_len: u32,
    control: *const u8,
    control_len: u32,
    flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AddressFamily(i32);

impl AddressFamily {
    pub const UNSPEC: Self = Self(0);
    pub const LOCAL: Self = Self::UNIX;
    pub const UNIX: Self = Self(1);
    pub const INET: Self = Self(2);
    pub const ROUTE: Self = Self(17);
    pub const INET6: Self = Self(28);
}

impl Display for AddressFamily {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Self::UNSPEC => write!(f, "UNSPEC"),
            Self::LOCAL => write!(f, "LOCAL"),
            Self::INET => write!(f, "INET"),
            Self::ROUTE => write!(f, "ROUTE"),
            Self::INET6 => write!(f, "INET6"),
            _ => todo!(),
        }
    }
}

enum SockOpt {}

#[derive(Debug, Error, Errno)]
enum SetOptError {
    #[error("Invalid value or value length")]
    #[errno(EFAULT)]
    InvalidValue,

    #[error("Invalid length")]
    #[errno(EINVAL)]
    InvalidLength,
}

#[derive(Debug, Error, Errno)]
enum GetOptError {
    #[error("invalid length")]
    #[errno(EFAULT)]
    InvalidValue,
}

#[derive(Debug, Error, Errno)]
enum SendItError {}
