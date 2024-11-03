// SPDX-License-Identifier: MIT OR Apache-2.0
use super::ram::Ram;
use std::error::Error;

pub use self::arch::*;
pub use self::os::new;

use gdbstub::stub::MultiThreadStopReason;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
#[cfg_attr(target_os = "linux", path = "linux/mod.rs")]
#[cfg_attr(target_os = "macos", path = "macos/mod.rs")]
#[cfg_attr(target_os = "windows", path = "windows/mod.rs")]
mod os;

#[cfg(target_os = "linux")]
pub type Default = self::os::Kvm;

#[cfg(target_os = "macos")]
pub type Default = self::os::Hvf;

#[cfg(target_os = "windows")]
pub type Default = self::os::Whp;

#[cfg(target_os = "linux")]
pub type DefaultError = self::os::KvmError;

#[cfg(target_os = "macos")]
pub type DefaultError = crate::vmm::VmmError;

#[cfg(target_os = "windows")]
pub type DefaultError = crate::vmm::VmmError;

/// Underlying hypervisor (e.g. KVM on Linux).
pub trait Hypervisor: Send + Sync + 'static {
    type Cpu<'a>: CpuRun
    where
        Self: 'a;
    type CpuErr: Error + Send + 'static;

    fn cpu_features(&self) -> &CpuFeats;
    fn ram(&self) -> &Ram;
    fn ram_mut(&mut self) -> &mut Ram;

    /// This method must be called by a thread that is going to drive the returned CPU.
    fn create_cpu(&self, id: usize) -> Result<Self::Cpu<'_>, Self::CpuErr>;
}

/// Represents a core of the PS4 CPU.
///
/// On AArch64 this represent one Processing Element (PE).
pub trait Cpu {
    type States<'a>: CpuCommit
    where
        Self: 'a;
    type GetStatesErr: Error + Send + 'static;
    type Exit<'a>: CpuExit<Cpu = Self>
    where
        Self: 'a;
    type TranslateErr: Error + Send + 'static;

    fn id(&self) -> usize;

    fn states(&mut self) -> Result<Self::States<'_>, Self::GetStatesErr>;

    fn translate(&self, vaddr: usize) -> Result<usize, Self::TranslateErr>;
}

/// Provides a method to run the CPU.
pub trait CpuRun: Cpu {
    type RunErr: Error + Send + 'static;

    fn run(&mut self) -> Result<Self::Exit<'_>, Self::RunErr>;
}

/// Provides a method to commit [`CpuStates`].
pub trait CpuCommit: CpuStates {
    fn commit(self) -> Result<(), Self::Err>;
}

/// Contains information when VM exited.
pub trait CpuExit: Sized {
    type Cpu: Cpu;
    type Io: CpuIo<Cpu = Self::Cpu>;
    type Debug: CpuDebug;

    fn cpu(&mut self) -> &mut Self::Cpu;
    #[cfg(target_arch = "x86_64")]
    fn into_hlt(self) -> Result<(), Self>;
    fn into_io(self) -> Result<Self::Io, Self>;
    fn into_debug(self) -> Result<Self::Debug, Self>;
}

/// Contains information when a VM exited because of memory-mapped I/O.
pub trait CpuIo {
    type Cpu: Cpu;

    /// Returns physical address where the VM try to access.
    fn addr(&self) -> usize;
    fn buffer(&mut self) -> IoBuf;
    fn cpu(&mut self) -> &mut Self::Cpu;
}

/// Encapsulates a buffer for memory-mapped I/O.
pub enum IoBuf<'a> {
    Write(&'a [u8]),
    Read(&'a mut [u8]),
}

/// Contains information when a VM exited because of debug event.
pub trait CpuDebug {
    type Cpu: Cpu;

    fn reason(&mut self) -> MultiThreadStopReason<u64>;

    fn cpu(&mut self) -> &mut Self::Cpu;
}
