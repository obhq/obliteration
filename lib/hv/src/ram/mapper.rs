use std::error::Error;
use std::num::NonZero;

/// Provides a method to map a portion of RAM dynamically.
pub(crate) trait RamMapper: Send + Sync + 'static {
    /// # Safety
    /// The range specified with `host` and `len` will be shared with the VM, which mean it should
    /// not contains any data that should not visible to the VM.
    unsafe fn map(
        &self,
        host: *mut u8,
        vm: usize,
        len: NonZero<usize>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}

impl RamMapper for () {
    unsafe fn map(
        &self,
        _: *mut u8,
        _: usize,
        _: NonZero<usize>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
}
