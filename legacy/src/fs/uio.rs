use crate::errno::{Errno, EINVAL};
use crate::syscalls::{SysArg, SysOut};
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::num::NonZeroI32;
use std::ops::{Deref, DerefMut};
use thiserror::Error;

/// Implementation of `iovec` structure for writing.
#[repr(C)]
pub struct IoVec<'a> {
    ptr: *const u8,
    len: IoLen,
    phantom: PhantomData<&'a [u8]>,
}

impl<'a> IoVec<'a> {
    /// # Safety
    /// `ptr` must outlive `'a`.
    pub unsafe fn new(ptr: *const u8, len: IoLen) -> Self {
        Self {
            ptr,
            len,
            phantom: PhantomData,
        }
    }

    pub fn len(&self) -> IoLen {
        self.len
    }
}

impl<'a> Deref for IoVec<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len.get()) }
    }
}

/// Implementation of `iovec` structure for reading.
#[repr(C)]
pub struct IoVecMut<'a> {
    ptr: *mut u8,
    len: IoLen,
    phantom: PhantomData<&'a mut [u8]>,
}

impl<'a> IoVecMut<'a> {
    /// # Safety
    /// `ptr` must outlive `'a`.
    pub unsafe fn new(ptr: *mut u8, len: IoLen) -> Self {
        Self {
            ptr,
            len,
            phantom: PhantomData,
        }
    }

    pub fn len(&self) -> IoLen {
        self.len
    }
}

impl<'a> Deref for IoVecMut<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len.get()) }
    }
}

impl<'a> DerefMut for IoVecMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len.get()) }
    }
}

/// Represents a length of [`IoVec`] and [`IoVecMut`].
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IoLen(usize);

impl IoLen {
    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(0x7fffffff);

    pub fn from_usize(v: usize) -> Result<Self, IoLenError> {
        let v = Self(v);

        if v > Self::MAX {
            Err(IoLenError(()))
        } else {
            Ok(v)
        }
    }

    pub fn get(self) -> usize {
        self.0
    }

    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        let r = self.0.checked_add(rhs.0).map(IoLen)?;

        if r > Self::MAX {
            None
        } else {
            Some(r)
        }
    }

    pub fn saturating_add(self, rhs: Self) -> Self {
        let r = Self(self.0.saturating_add(rhs.0));

        if r > Self::MAX {
            Self::MAX
        } else {
            r
        }
    }

    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(IoLen)
    }
}

impl TryFrom<SysArg> for IoLen {
    type Error = IoLenError;

    fn try_from(value: SysArg) -> Result<Self, Self::Error> {
        Self::from_usize(value.get())
    }
}

impl PartialEq<usize> for IoLen {
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

impl Display for IoLen {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<IoLen> for SysOut {
    fn from(value: IoLen) -> Self {
        value.0.into()
    }
}

/// Represents an error when [`IoLen`] fails to construct.
#[derive(Debug, Error)]
#[error("invalid value")]
pub struct IoLenError(());

impl Errno for IoLenError {
    fn errno(&self) -> NonZeroI32 {
        EINVAL
    }
}
