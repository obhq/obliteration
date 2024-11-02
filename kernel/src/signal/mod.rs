/// Value of *nix signal.
#[derive(Debug, Clone, Copy)]
pub struct Signal(u8);

impl Signal {
    const MAX: u8 = 128; // _SIG_MAXSIG

    /// # Panics
    /// If `v` is not a valid signal number.
    pub const fn from_bits(v: u8) -> Self {
        assert!(v <= Self::MAX);
        Self(v)
    }

    pub const fn into_bits(self) -> u8 {
        self.0
    }
}
