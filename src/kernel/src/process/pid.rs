use crate::syscalls::SysOut;
use std::num::NonZeroI32;

/// Unique identifier of a process.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pid(i32);

impl Pid {
    pub const KERNEL: Self = Self(0);
}

impl From<NonZeroI32> for Pid {
    fn from(value: NonZeroI32) -> Self {
        Self(value.get())
    }
}

impl PartialEq<i32> for Pid {
    fn eq(&self, other: &i32) -> bool {
        self.0 == *other
    }
}

impl PartialEq<Pid> for i32 {
    fn eq(&self, other: &Pid) -> bool {
        *self == other.0
    }
}

impl From<Pid> for SysOut {
    fn from(value: Pid) -> Self {
        value.0.into()
    }
}
