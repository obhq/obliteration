use crate::errno::{Errno, EINVAL};
use std::num::NonZeroI32;
use thiserror::Error;

/// A registry of system parameters.
///
/// This is an implementation of
/// https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/kern/kern_sysctl.c.
pub struct Sysctl {}

impl Sysctl {
    pub const CTL_VM: i32 = 2;
    pub const CTL_DEBUG: i32 = 5;
    pub const VM_TOTAL: i32 = 1;

    pub fn new() -> Self {
        Self {}
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

        todo!("sysctl {top}");
    }
}

/// Represents an error for sysctl invocation.
#[derive(Debug, Error)]
pub enum InvokeError {
    #[error("name is not valid")]
    InvalidName,

    #[error("the process is not a system process")]
    NotSystem,
}

impl Errno for InvokeError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::InvalidName | Self::NotSystem => EINVAL,
        }
    }
}
