use alloc::string::String;
use alloc::sync::Arc;

pub fn identify_cpu() -> CpuInfo {
    todo!()
}

pub unsafe fn setup_main_cpu(cpu: CpuInfo) -> Arc<ArchConfig> {
    todo!()
}

/// Contains information for CPU on current machine.
pub struct CpuInfo {
    pub cpu_vendor: String,
}

/// Contains architecture-specific configurations obtained from [`setup_main_cpu()`].
pub struct ArchConfig {
    pub secondary_start: &'static [u8],
}
