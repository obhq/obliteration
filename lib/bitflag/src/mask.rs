use crate::Type;
use core::marker::PhantomData;
use core::ops::{BitOr, Not};

/// Mask of a value in the set.
pub struct Mask<T: Type, V> {
    mask: T::Raw,
    phantom: PhantomData<V>,
}

impl<T: Type, V> Mask<T, V> {
    /// # Safety
    /// `mask` must be valid for `V` (e.g. has exactly one bit set if `V` is `bool`) and must not
    /// have zero bit in the middle.
    pub const unsafe fn new(mask: T::Raw) -> Self {
        Self {
            mask,
            phantom: PhantomData,
        }
    }

    pub const fn mask(self) -> T::Raw {
        self.mask
    }
}

impl<T: Type, V> Clone for Mask<T, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Type, V> Copy for Mask<T, V> {}

impl<T: Type> BitOr for Mask<T, bool> {
    type Output = T;

    fn bitor(self, rhs: Self) -> Self::Output {
        T::from(self.mask | rhs.mask)
    }
}

impl<T: Type, V> Not for Mask<T, V> {
    type Output = T;

    fn not(self) -> Self::Output {
        T::from(!self.mask)
    }
}
