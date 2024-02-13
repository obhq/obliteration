use super::{strsignal, Signal, SignalIter};
use std::fmt::{Display, Formatter};
use std::ops::{BitAndAssign, BitOrAssign, Not};

/// An implementation of `sigset_t`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SignalSet {
    bits: [u32; 4],
}

impl SignalSet {
    /// An implementation of `SIGISMEMBER`.
    pub fn contains(&self, sig: Signal) -> bool {
        (self.bits[Self::word(sig)] & Self::bit(sig)) != 0
    }

    /// An implementation of `SIGADDSET`.
    pub fn add(&mut self, sig: Signal) {
        self.bits[Self::word(sig)] |= Self::bit(sig);
    }

    /// An implementation of `SIGDELSET`.
    pub fn remove(&mut self, sig: Signal) {
        self.bits[Self::word(sig)] &= !Self::bit(sig);
    }

    // An implementation of `_SIG_IDX`.
    fn idx(s: Signal) -> i32 {
        s.get() - 1
    }

    /// An implementation of `_SIG_WORD`.
    fn word(s: Signal) -> usize {
        // This is safe because `Signal` is guaranteed to be non-negative.
        unsafe { (Self::idx(s) >> 5).try_into().unwrap_unchecked() }
    }

    /// An implementation of `_SIG_BIT`.
    fn bit(s: Signal) -> u32 {
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

        for sig in SignalIter::new().filter(|sig| self.contains(*sig)) {
            if !first {
                f.write_str(" | ")?;
            }

            f.write_str(strsignal(sig).as_ref())?;
            first = false;
        }

        if first {
            f.write_str("none")?;
        }

        Ok(())
    }
}
