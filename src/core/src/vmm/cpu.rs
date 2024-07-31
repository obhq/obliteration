use std::error::Error;

/// Represents a core of the PS4 CPU.
pub trait Cpu {
    type States<'a>: CpuStates + 'a
    where
        Self: 'a;
    type GetStatesErr: Error + Send + 'static;
    type Exit<'a>: CpuExit + 'a
    where
        Self: 'a;
    type RunErr: Error + Send + 'static;

    fn id(&self) -> usize;
    fn states(&mut self) -> Result<Self::States<'_>, Self::GetStatesErr>;
    fn run(&mut self) -> Result<Self::Exit<'_>, Self::RunErr>;
}

/// States of [`Cpu`].
///
/// [`Drop`] implementation on the type that implement this trait may panic if it fails to commit
/// the states.
pub trait CpuStates {
    #[cfg(target_arch = "x86_64")]
    fn set_rsp(&mut self, v: usize);

    #[cfg(target_arch = "x86_64")]
    fn set_rip(&mut self, v: usize);

    #[cfg(target_arch = "x86_64")]
    fn set_cr0(&mut self, v: usize);

    #[cfg(target_arch = "x86_64")]
    fn set_cr3(&mut self, v: usize);

    #[cfg(target_arch = "x86_64")]
    fn set_cr4(&mut self, v: usize);

    #[cfg(target_arch = "x86_64")]
    fn set_efer(&mut self, v: usize);

    #[cfg(target_arch = "x86_64")]
    fn set_cs(&mut self, ty: u8, dpl: u8, p: bool, l: bool, d: bool);

    #[cfg(target_arch = "x86_64")]
    fn set_ds(&mut self, p: bool);

    #[cfg(target_arch = "x86_64")]
    fn set_es(&mut self, p: bool);

    #[cfg(target_arch = "x86_64")]
    fn set_fs(&mut self, p: bool);

    #[cfg(target_arch = "x86_64")]
    fn set_gs(&mut self, p: bool);

    #[cfg(target_arch = "x86_64")]
    fn set_ss(&mut self, p: bool);


    #[cfg(target_arch = "aarch64")]
    fn set_sp(&mut self, v: usize);

    #[cfg(target_arch = "aarch64")]
    fn set_pc(&mut self, v: usize);
}

/// Contains information when VM exited.
pub trait CpuExit {
    #[cfg(target_arch = "x86_64")]
    fn reason(&mut self) -> ExitReason;
}

#[derive(Debug)]
pub enum ExitReason<'a> {
    Hlt,
    Other,
    IoOut(u16, &'a [u8]),
}
