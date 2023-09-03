use crate::arc4::Arc4;
use crate::errno::{Errno, EINVAL, ESRCH};
use crate::process::{AppInfoReadError, VProc};
use std::cmp::min;
use std::num::NonZeroI32;
use thiserror::Error;

/// A registry of system parameters.
///
/// This is an implementation of
/// https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/kern/kern_sysctl.c.
pub struct Sysctl {
    arc4: &'static Arc4,
    vp: &'static VProc,
}

impl Sysctl {
    pub const CTL_KERN: i32 = 1;
    pub const CTL_VM: i32 = 2;
    pub const CTL_DEBUG: i32 = 5;
    pub const KERN_PROC: i32 = 14;
    pub const KERN_ARND: i32 = 37;
    pub const KERN_PROC_APPINFO: i32 = 35;
    pub const VM_TOTAL: i32 = 1;

    pub fn new(arc4: &'static Arc4, vp: &'static VProc) -> Self {
        Self { arc4, vp }
    }

    pub fn invoke(
        &self,
        name: &[i32],
        old: Option<&mut [u8]>,
        new: Option<&[u8]>,
    ) -> Result<usize, InvokeError> {
        // Check arguments.
        if name.len() < 2 || name.len() > 24 {
            return Err(InvokeError::InvalidName);
        }

        // Check top-level number.
        let top = name[0];

        if top == Self::CTL_DEBUG {
            return Err(InvokeError::NotSystem);
        } else if top == Self::CTL_VM && name[1] == Self::VM_TOTAL {
            todo!("sysctl CTL_VM:VM_TOTAL")
        }

        // TODO: Check userland_sysctl to see what we have missed here.
        match top {
            Self::CTL_KERN => self.invoke_kern(&name[1..], old, new),
            v => todo!("sysctl {v}"),
        }
    }

    fn invoke_kern(
        &self,
        name: &[i32],
        old: Option<&mut [u8]>,
        new: Option<&[u8]>,
    ) -> Result<usize, InvokeError> {
        match name[0] {
            Self::KERN_PROC => match name[1] {
                Self::KERN_PROC_APPINFO => self.kern_proc_appinfo(&name[2..], old, new),
                v => todo!("sysctl CTL_KERN:KERN_PROC:{v}"),
            },
            Self::KERN_ARND => self.kern_arnd(old),
            v => todo!("sysctl CTL_KERN:{v}"),
        }
    }

    fn kern_proc_appinfo(
        &self,
        name: &[i32],
        old: Option<&mut [u8]>,
        new: Option<&[u8]>,
    ) -> Result<usize, InvokeError> {
        // Check if the request is for our process.
        if name[0] != self.vp.id().get() {
            return Err(InvokeError::InvalidAppInfoPid);
        }

        // Get the info.
        let old = match old {
            Some(v) => {
                if let Err(e) = self.vp.app_info().read(v) {
                    return Err(InvokeError::ReadAppInfoFailed(e));
                }

                v.len()
            }
            None => 0,
        };

        // Update the info.
        if new.is_some() {
            todo!("sysctl CTL_KERN:KERN_PROC:KERN_PROC_APPINFO with non-null new");
        }

        Ok(old)
    }

    fn kern_arnd(&self, old: Option<&mut [u8]>) -> Result<usize, InvokeError> {
        // Get output buffer.
        let buf = match old {
            Some(v) => v,
            None => {
                // TODO: Check how PS4 handle this case.
                return Ok(0);
            }
        };

        // Fill the output.
        let len = min(buf.len(), 256);

        self.arc4.rand_bytes(&mut buf[..len]);

        Ok(len)
    }
}

/// Represents an error for sysctl invocation.
#[derive(Debug, Error)]
pub enum InvokeError {
    #[error("name is not valid")]
    InvalidName,

    #[error("the process is not a system process")]
    NotSystem,

    #[error("pid is not valid for app info")]
    InvalidAppInfoPid,

    #[error("cannot read app info")]
    ReadAppInfoFailed(#[source] AppInfoReadError),
}

impl Errno for InvokeError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::InvalidName | Self::NotSystem => EINVAL,
            Self::InvalidAppInfoPid => ESRCH,
            Self::ReadAppInfoFailed(e) => e.errno(),
        }
    }
}
