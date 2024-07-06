use crate::syscalls::{SysArg, SysOut};
use std::borrow::Borrow;

/// Unique identifier of a process.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pid(i32);

impl Pid {
    pub const KERNEL: Self = Self(0);

    /// Returns [`None`] if `v` is negative.
    pub const fn new(v: i32) -> Option<Self> {
        if v >= 0 {
            Some(Self(v))
        } else {
            None
        }
    }
}

impl From<SysArg> for Pid {
    fn from(value: SysArg) -> Self {
        // We want to catch when the PS4 send an unexpected PID instead of silently return an error.
        Pid::new(value.try_into().unwrap()).unwrap()
    }
}

impl Borrow<i32> for Pid {
    fn borrow(&self) -> &i32 {
        &self.0
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
