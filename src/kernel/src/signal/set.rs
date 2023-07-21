use std::num::NonZeroI32;
use std::ops::{BitAndAssign, BitOrAssign, Not};

/// An implementation of `sigset_t`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SignalSet {
    bits: [u32; 4],
}

impl SignalSet {
    /// An implementation of `SIGDELSET`.
    pub fn remove(&mut self, sig: NonZeroI32) {
        self.bits[Self::word(sig)] &= !Self::bit(sig);
    }

    // An implementation of `_SIG_IDX`.
    fn idx(s: NonZeroI32) -> i32 {
        s.get() - 1
    }

    /// An implementation of `_SIG_WORD`.
    fn word(s: NonZeroI32) -> usize {
        (Self::idx(s) >> 5).try_into().unwrap()
    }

    /// An implementation of `_SIG_BIT`.
    fn bit(s: NonZeroI32) -> u32 {
        1 << (Self::idx(s) & 31)
    }
}

impl Default for SignalSet {
    fn default() -> Self {
        Self { bits: [0u32; 4] }
    }
}

impl BitAndAssign for SignalSet {
    fn bitand_assign(&mut self, rhs: Self) {
        for i in 0..4 {
            self.bits[i] &= rhs.bits[i];
        }
    }
}

impl BitOrAssign for SignalSet {
    fn bitor_assign(&mut self, rhs: Self) {
        for i in 0..4 {
            self.bits[i] |= rhs.bits[i];
        }
    }
}

impl Not for SignalSet {
    type Output = Self;

    fn not(mut self) -> Self::Output {
        for i in 0..4 {
            self.bits[i] = !self.bits[i];
        }
        self
    }
}
