use super::strsignal;
use std::fmt::{Display, Formatter};
use std::num::NonZeroI32;
use std::ops::{BitAndAssign, BitOrAssign, Not};

/// An implementation of `sigset_t`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SignalSet {
    bits: [u32; 4],
}

impl SignalSet {
    /// An implementation of `SIGISMEMBER`.
    pub fn contains(&self, sig: NonZeroI32) -> bool {
        (self.bits[Self::word(sig)] & Self::bit(sig)) != 0
    }

    /// An implementation of `SIGADDSET`.
    pub fn add(&mut self, sig: NonZeroI32) {
        self.bits[Self::word(sig)] |= Self::bit(sig);
    }

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

impl Display for SignalSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut first = true;

        for i in 1..=128 {
            let num = unsafe { NonZeroI32::new_unchecked(i) };

            if self.contains(num) {
                if !first {
                    f.write_str(" | ")?;
                }

                f.write_str(strsignal(num).as_ref())?;
                first = false;
            }
        }

        if first {
            f.write_str("none")?;
        }

        Ok(())
    }
}
