use std::num::NonZeroI32;

/// Outputs of the syscall.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Output {
    pub rax: usize,
    pub rdx: usize,
}

impl Output {
    pub const ZERO: Output = Output { rax: 0, rdx: 0 };
}

impl<T> From<*mut T> for Output {
    fn from(value: *mut T) -> Self {
        Self {
            rax: value as _,
            rdx: 0,
        }
    }
}

impl From<i32> for Output {
    fn from(value: i32) -> Self {
        Self {
            rax: value as isize as usize,
            rdx: 0,
        }
    }
}

impl From<usize> for Output {
    fn from(value: usize) -> Self {
        Self { rax: value, rdx: 0 }
    }
}

impl From<NonZeroI32> for Output {
    fn from(value: NonZeroI32) -> Self {
        Self {
            rax: value.get() as isize as usize,
            rdx: 0,
        }
    }
}
