use core::num::NonZero;

/// Provides boot information when booting on a Virtual Machine.
#[repr(C)]
pub struct Vm {
    /// Physical address of one page for console memory.
    pub console: usize,
    /// Page size on the host.
    pub host_page_size: NonZero<usize>,
}
