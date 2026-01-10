use crate::config::Config;
use alloc::string::String;
use alloc::sync::Arc;
use config::KernelMap;

pub fn identify_cpu() -> CpuInfo {
    todo!()
}

pub unsafe fn setup_main_cpu(
    config: &Config,
    cpu: CpuInfo,
    map: &'static KernelMap,
) -> Arc<ArchConfig> {
    todo!()
}

/// Contains information for CPU on current machine.
pub struct CpuInfo {
    pub cpu_vendor: String,
    pub cpu_id: u32, // TODO: Figure out how to remove this.
}

/// Contains architecture-specific configurations obtained from [`setup_main_cpu()`].
pub struct ArchConfig {
    pub secondary_start: &'static [u8],
}
