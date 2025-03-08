use std::error::Error;
use std::num::NonZero;

/// Provides methods to map a portion of RAM dynamically.
pub trait RamMapper: Send + Sync {
    type Err: Error + Send + Sync + 'static;

    /// # Safety
    /// The range specified with `host` and `len` will be shared with the VM, which mean it should
    /// not contains any data that should not visible to the VM.
    unsafe fn map(&self, host: *mut u8, vm: usize, len: NonZero<usize>) -> Result<(), Self::Err>;
}
