use super::Cpu;
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
