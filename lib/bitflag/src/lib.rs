#![no_std]

pub use self::mask::*;

use core::ops::{BitOr, Not};

mod mask;

/// Type of bit flag.
pub trait Type: From<Self::Raw> {
    type Raw: Raw;
}

/// Underlying type of [`Type`].
pub trait Raw: Eq + BitOr<Output = Self> + Not<Output = Self> + Copy {}

impl Raw for u32 {}

/// Provides method to construct a value from [`Raw`].
pub trait FromRaw<T>: Sized {
    fn from_raw(raw: T) -> Option<Self>;
}

impl FromRaw<u32> for u16 {
    fn from_raw(raw: u32) -> Option<Self> {
        raw.try_into().ok()
    }
}
