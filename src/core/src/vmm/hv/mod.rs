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
///
/// On AArch64 this represent one Processing Element (PE).
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

    /// # Panics
    /// - If `ips` greater than 7.
    /// - If `t0sz` or `t1sz` larger than 6 bits.
    #[cfg(target_arch = "aarch64")]
    fn set_tcr_el1(&mut self, ips: u8, a1: bool, t0sz: u8, t1sz: u8);

    /// # Panics
    /// If `baddr` has non-zero on bit 0 or 48:64.
    #[cfg(target_arch = "aarch64")]
    fn set_ttbr1_el1(&mut self, baddr: usize);

    #[cfg(target_arch = "aarch64")]
    fn set_sp_el1(&mut self, v: usize);

    #[cfg(target_arch = "aarch64")]
    fn set_pc(&mut self, v: usize);

    fn commit(self) -> Result<(), Self::Err>;
}

/// Contains information when VM exited.
pub trait CpuExit: Sized {
    type Io: CpuIo;

    #[cfg(target_arch = "x86_64")]
    fn into_hlt(self) -> Result<(), Self>;

    fn into_io(self) -> Result<Self::Io, Self>;
}

/// Contains information when a VM exited because of memory-mapped I/O.
pub trait CpuIo {
    /// Returns physical address where the VM attempting to be accessed.
    fn addr(&self) -> usize;
    fn buffer(&mut self) -> IoBuf;
    fn translate(&self, vaddr: usize) -> Result<usize, Box<dyn Error>>;
}

/// Encapsulates a buffer for memory-mapped I/O.
pub enum IoBuf<'a> {
    Write(&'a [u8]),
    Read(&'a mut [u8]),
}
