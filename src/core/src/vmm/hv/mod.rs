use std::error::Error;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub type Default = self::linux::Kvm;

#[cfg(target_os = "windows")]
pub type Default = self::windows::Whp;

#[cfg(target_os = "macos")]
pub type Default = self::macos::Hf;

/// Underlying hypervisor (e.g. KVM on Linux).
pub trait Hypervisor: Send + Sync {
    type Cpu<'a>: Cpu
    where
        Self: 'a;
    type CpuErr: Error + Send;

    /// This method must be called by a thread that is going to drive the returned CPU.
    fn create_cpu(&self, id: usize) -> Result<Self::Cpu<'_>, Self::CpuErr>;
}

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

    fn states(&mut self) -> Result<Self::States<'_>, Self::GetStatesErr>;
    fn run(&mut self) -> Result<Self::Exit<'_>, Self::RunErr>;
}

/// States of [`Cpu`].
pub trait CpuStates {
    type Err: Error + Send + 'static;

    #[cfg(target_arch = "x86_64")]
    fn set_rdi(&mut self, v: usize);

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

    fn commit(self) -> Result<(), Self::Err>;
}

/// Contains information when VM exited.
pub trait CpuExit {
    #[cfg(target_arch = "x86_64")]
    fn is_hlt(&self) -> bool;

    #[cfg(target_arch = "x86_64")]
    fn is_io(&mut self) -> Option<CpuIo>;
}

/// Contains information when a VM exited because of I/O instructions.
#[cfg(target_arch = "x86_64")]
pub enum CpuIo<'a> {
    Out(u16, &'a [u8]),
}
