/// Provides boot information when booting on a Virtual Machine.
#[repr(C)]
pub struct Vm {
    /// Physical address of one page for console memory.
    pub console: usize,
}
