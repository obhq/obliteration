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
