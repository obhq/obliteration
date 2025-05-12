use alloc::string::String;
use alloc::sync::Arc;

pub fn cpu_model() -> String {
    todo!()
}

pub unsafe fn setup_main_cpu() -> Arc<ArchConfig> {
    todo!()
}

/// Contains architecture-specific configurations obtained from [`setup_main_cpu()`].
pub struct ArchConfig {
    pub secondary_start: &'static [u8],
}
