use core::borrow::Borrow;
use core::ffi::c_int;

/// Unique identifier of a process.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pid(c_int);

impl Pid {
    pub const KERNEL: Self = Self(0);
    pub const IDLE: Self = Self(10);

    /// Returns [`None`] if `v` is negative.
    pub const fn new(v: c_int) -> Option<Self> {
        if v >= 0 { Some(Self(v)) } else { None }
    }
}

impl Borrow<c_int> for Pid {
    fn borrow(&self) -> &c_int {
        &self.0
    }
}

impl PartialEq<c_int> for Pid {
    fn eq(&self, other: &c_int) -> bool {
        self.0 == *other
    }
}

impl PartialEq<Pid> for c_int {
    fn eq(&self, other: &Pid) -> bool {
        *self == other.0
    }
}
