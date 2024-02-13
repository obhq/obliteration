use crate::budget::BudgetType;
use crate::fs::{VFileFlags, VFileType};
use crate::{
    process::VThread,
    syscalls::{SysErr, SysIn, SysOut, Syscalls},
};
use core::fmt;
use std::{
    fmt::{Display, Formatter},
    sync::Arc,
};

pub use self::socket::*;

mod socket;

pub struct NetManager {}

impl NetManager {
    pub fn new(sys: &mut Syscalls) -> Arc<Self> {
        let net = Arc::new(Self {});

        sys.register(97, &net, Self::sys_socket);
        sys.register(113, &net, Self::sys_socketex);

        net
    }

    fn sys_socket(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let domain: i32 = i.args[0].try_into().unwrap();
        let ty: i32 = i.args[1].try_into().unwrap();
        let proto: i32 = i.args[2].try_into().unwrap();

        let td = VThread::current().unwrap();

        let budget = if domain == 1 {
            BudgetType::FdIpcSocket
        } else {
            BudgetType::FdSocket
        };

        let fd = td.proc().files().alloc_with_budget::<SocketCreateError>(
            |_| {
                let so = Socket::new(domain, ty, proto, td.as_ref().cred(), td.as_ref(), None)?;

                let ty = if domain == 1 {
                    VFileType::IpcSocket(so)
                } else {
                    VFileType::Socket(so)
                };

                Ok(ty)
            },
            VFileFlags::FWRITE | VFileFlags::FREAD,
            budget,
        )?;

        Ok(fd.into())
    }

    fn sys_socketex(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let name = unsafe { i.args[0].to_str(32)? };
        let domain: i32 = i.args[1].try_into().unwrap();
        let ty: i32 = i.args[2].try_into().unwrap();
        let proto: i32 = i.args[3].try_into().unwrap();

        let td = VThread::current().unwrap();

        let budget = if domain == 1 {
            BudgetType::FdIpcSocket
        } else {
            BudgetType::FdSocket
        };

        let fd = td.proc().files().alloc_with_budget::<SocketCreateError>(
            |_| {
                let so = Socket::new(domain, ty, proto, td.as_ref().cred(), td.as_ref(), name)?;

                let ty = if domain == 1 {
                    VFileType::IpcSocket(so)
                } else {
                    VFileType::Socket(so)
                };

                Ok(ty)
            },
            VFileFlags::FWRITE | VFileFlags::FREAD,
            budget,
        )?;

        Ok(fd.into())
    }
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
