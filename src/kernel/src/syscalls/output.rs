use std::num::NonZeroI32;

use crate::fs::Fd;

/// Outputs of the syscall entry point.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SysOut {
    rax: usize,
    rdx: usize,
}

impl SysOut {
    pub const ZERO: Self = Self { rax: 0, rdx: 0 };
}

impl<T> From<*mut T> for SysOut {
    fn from(value: *mut T) -> Self {
        Self {
            rax: value as _,
            rdx: 0,
        }
    }
}

impl From<i32> for SysOut {
    fn from(value: i32) -> Self {
        Self {
            rax: value as isize as usize, // Sign extended.
            rdx: 0,
        }
    }
}

impl From<usize> for SysOut {
    fn from(value: usize) -> Self {
        Self { rax: value, rdx: 0 }
    }
}

impl From<NonZeroI32> for SysOut {
    fn from(value: NonZeroI32) -> Self {
        Self {
            rax: value.get() as isize as usize, // Sign extended.
            rdx: 0,
        }
    }
}
