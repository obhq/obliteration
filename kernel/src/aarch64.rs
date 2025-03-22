use alloc::sync::Arc;

pub unsafe fn setup_main_cpu() -> Arc<ArchConfig> {
    todo!()
}

/// Contains architecture-specific configurations obtained from [`setup_main_cpu()`].
pub struct ArchConfig {
    pub secondary_start: &'static [u8],
}
