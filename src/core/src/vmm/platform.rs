use super::Cpu;
use std::error::Error;

/// Underlying hypervisor (e.g. KVM on Linux).
pub trait Platform {
    type Cpu<'a>: Cpu
    where
        Self: 'a;
    type CpuErr: Error;

    fn create_cpu(&self, id: usize) -> Result<Self::Cpu<'_>, Self::CpuErr>;
}
