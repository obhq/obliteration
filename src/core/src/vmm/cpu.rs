use std::error::Error;

/// Represents a core of the PS4 CPU.
pub trait Cpu {
    type States<'a>: CpuStates + 'a
    where
        Self: 'a;
    type GetStatesErr: Error + Send + 'static;

    fn id(&self) -> usize;
    fn states(&mut self) -> Result<Self::States<'_>, Self::GetStatesErr>;
}

/// States of [`Cpu`].
///
/// [`Drop`] implementation on the type that implement this trait may panic if it fails to commit
/// the states.
pub trait CpuStates {
    #[cfg(target_arch = "x86_64")]
    fn set_cr0(&mut self, v: usize);

    #[cfg(target_arch = "x86_64")]
    fn set_cr3(&mut self, v: usize);

    #[cfg(target_arch = "x86_64")]
    fn set_cr4(&mut self, v: usize);

    #[cfg(target_arch = "x86_64")]
    fn set_efer(&mut self, v: usize);
}
