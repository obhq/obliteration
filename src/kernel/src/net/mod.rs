use crate::budget::BudgetType;
use crate::errno::{Errno, EFAULT, EINVAL, ENAMETOOLONG};
use crate::fs::{IoVec, VFile, VFileFlags, VFileType};
use crate::process::GetSocketError;
use crate::{arnd, info};
use crate::{
    process::VThread,
    syscalls::{SysErr, SysIn, SysOut, Syscalls},
};
use bitflags::bitflags;
use macros::Errno;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

pub use self::socket::*;

mod proto;
mod socket;

pub struct NetManager {}

impl NetManager {
    pub fn new(sys: &mut Syscalls) -> Arc<Self> {
        let net = Arc::new(Self {});

        sys.register(27, &net, Self::sys_recvmsg);
        sys.register(28, &net, Self::sys_sendmsg);
        sys.register(29, &net, Self::sys_recvfrom);
        sys.register(30, &net, Self::sys_accept);
        sys.register(97, &net, Self::sys_socket);
        sys.register(98, &net, Self::sys_connect);
        sys.register(99, &net, Self::sys_netcontrol);
        sys.register(104, &net, Self::sys_bind);
        sys.register(105, &net, Self::sys_setsockopt);
        sys.register(106, &net, Self::sys_listen);
        sys.register(113, &net, Self::sys_socketex);
        sys.register(114, &net, Self::sys_socketclose);
        sys.register(118, &net, Self::sys_getsockopt);
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

    fn sys_accept(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let fd: i32 = i.args[0].try_into().unwrap();
        let addr: *mut u8 = i.args[1].into();
        let addrlen: *mut u32 = i.args[2].try_into().unwrap();

        todo!()
    }

    fn sys_bind(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let fd: i32 = i.args[0].try_into().unwrap();
        let ptr: *const u8 = i.args[1].into();
        let len: i32 = i.args[2].try_into().unwrap();

        let addr = SockAddr::get(ptr, len)?;

        self.bind(fd, &addr, td)?;

        Ok(SysOut::ZERO)
    }

    fn bind(&self, fd: i32, addr: &SockAddr, td: &VThread) -> Result<(), BindError> {
        let socket = td.proc().files().get_socket(fd)?;

        socket.bind(addr, td)?;

        Ok(())
    }

    fn sys_netcontrol(self: &Arc<Self>, _: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let fd: i32 = i.args[0].try_into().unwrap();
        let op: i32 = i.args[1].try_into().unwrap();
        let ptr: *mut u8 = i.args[2].into();
        let buflen: u32 = i.args[3].try_into().unwrap();

        info!("Netcontrol called with op = {op}.");

        let mut buf = if ptr.is_null() {
            None
        } else {
            if buflen > 160 {
                return Err(SysErr::Raw(EINVAL));
            }

            let buf = Box::new([0u8; 160]);

            if op & 0x30000000 != 0 {
                // TODO: copyin
                todo!()
            }

            Some(buf)
        };

        let _ = if fd < 0 {
        } else {
            todo!()
        };

        match buf.as_mut() {
            Some(buf) => match op {
                // bnet_get_secure_seed
                0x14 if buflen > 3 => arnd::rand_bytes(&mut buf[..4]),
                _ => todo!("netcontrol with op = {op}"),
            },
            None => todo!("netcontrol with buf = null"),
        }

        if fd > -1 {
            todo!()
        }

        if let Some(buf) = buf {
            if op & 0x30000000 != 0x20000000 {
                unsafe { std::ptr::copy_nonoverlapping(buf.as_ptr(), ptr, buflen as usize) };
            }
        }

        Ok(SysOut::ZERO)
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

    fn sys_listen(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let fd: i32 = i.args[0].try_into().unwrap();
        let backlog: i32 = i.args[1].try_into().unwrap();

        let socket = td.proc().files().get_socket(fd)?;

        socket.listen(backlog, Some(td))?;

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
                    VFileType::IpcSocket
                } else {
                    VFileType::Socket
                };

                Ok(VFile::new(
                    ty,
                    VFileFlags::READ | VFileFlags::WRITE,
                    None,
                    todo!(),
                ))
            },
            budget,
        )?;

        info!("Opened a socket at fd {fd}.");

        Ok(fd.into())
    }

    fn sys_connect(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let fd: i32 = i.args[0].try_into().unwrap();
        let ptr: *const u8 = i.args[1].into();
        let len: i32 = i.args[2].try_into().unwrap();

        let addr = SockAddr::get(ptr, len)?;

        self.connect(fd, &addr, td)?;

        Ok(SysOut::ZERO)
    }

    fn connect(&self, fd: i32, addr: &SockAddr, td: &VThread) -> Result<(), ConnectError> {
        let socket = td.proc().files().get_socket(fd)?;

        todo!()
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
                    VFileType::IpcSocket
                } else {
                    VFileType::Socket
                };

                Ok(VFile::new(
                    ty,
                    VFileFlags::READ | VFileFlags::WRITE,
                    None,
                    todo!(),
                ))
            },
            budget,
        )?;

        if let Some(name) = name {
            info!("Opened a socket with name = {name} at fd {fd}.");
        } else {
            info!("Opened a socket at fd {fd}.");
        }

        Ok(fd.into())
    }

    fn sys_socketclose(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let fd: i32 = i.args[0].try_into().unwrap();

        info!("Attempting to close socket at fd {fd}.");

        td.proc().files().free(fd)?;

        Ok(SysOut::ZERO)
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

        let msg = MsgHdr {
            name: to,
            len: tolen,
            iovec: todo!(),
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
        if value.is_null() && len != 0 {
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
struct MsgHdr<'a> {
    name: *const u8,
    len: u32,
    iovec: *const IoVec<'a>,
    iovec_len: u32,
    control: *const u8,
    control_len: u32,
    flags: u32,
}

pub struct SockAddr([u8]);

impl SockAddr {
    pub fn get(ptr: *const u8, len: i32) -> Result<Box<Self>, GetSockAddrError> {
        if len > 255 {
            return Err(GetSockAddrError::TooLong);
        }

        if len < 2 {
            return Err(GetSockAddrError::TooShort);
        }

        // This is ok if the conditions above are upheld
        let len = len as usize;

        let mut vec = Vec::with_capacity(len);

        unsafe {
            std::ptr::copy_nonoverlapping(ptr, vec.as_mut_ptr(), len);
        }

        Ok(unsafe { std::mem::transmute(vec.into_boxed_slice()) })
    }

    pub fn family(&self) -> u8 {
        self.0[1]
    }

    pub fn data(&self) -> &[u8] {
        &self.0[2..]
    }
}

enum SockOpt {}

#[derive(Debug, Error, Errno)]
enum ConnectError {
    #[error("failed to get socket")]
    GetSocketError(#[from] GetSocketError),

    #[error("failed to connect")]
    ConnectError(#[from] Box<dyn Errno>),
}

#[derive(Debug, Error, Errno)]
enum BindError {
    #[error("failed to get socket")]
    GetSocketError(#[from] GetSocketError),

    #[error("failed to bind")]
    BindError(#[from] Box<dyn Errno>),
}

#[derive(Debug, Error, Errno)]
enum SetOptError {
    #[error("invalid value or length")]
    #[errno(EFAULT)]
    InvalidValue,

    #[error("invalid length")]
    #[errno(EINVAL)]
    InvalidLength,
}

#[derive(Debug, Error, Errno)]
enum GetOptError {
    #[error("invalid length")]
    #[errno(EINVAL)]
    InvalidValue,
}

#[derive(Debug, Error, Errno)]
enum SendItError {}

#[derive(Debug, Error, Errno)]
pub enum GetSockAddrError {
    #[error("length too big")]
    #[errno(ENAMETOOLONG)]
    TooLong,

    #[error("too short")]
    #[errno(EINVAL)]
    TooShort,
}
