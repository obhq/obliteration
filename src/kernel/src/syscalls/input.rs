use crate::fs::path::VPathBuf;
use std::num::TryFromIntError;

/// Input of the syscall entry point.
#[repr(C)]
pub struct Input<'a> {
    pub id: u32,
    pub offset: usize,
    pub module: &'a VPathBuf,
    pub args: [Arg; 6],
}

/// An argument of the syscall.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Arg(usize);

impl<T> From<Arg> for *const T {
    fn from(v: Arg) -> Self {
        v.0 as _
    }
}

impl<T> From<Arg> for *mut T {
    fn from(v: Arg) -> Self {
        v.0 as _
    }
}

impl From<Arg> for usize {
    fn from(v: Arg) -> Self {
        v.0
    }
}

impl TryFrom<Arg> for i32 {
    type Error = TryFromIntError;

    fn try_from(v: Arg) -> Result<Self, Self::Error> {
        v.0.try_into()
    }
}

impl TryFrom<Arg> for u32 {
    type Error = TryFromIntError;

    fn try_from(v: Arg) -> Result<Self, Self::Error> {
        v.0.try_into()
    }
}
