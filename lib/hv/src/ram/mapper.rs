use std::error::Error;
use std::num::NonZero;

/// Provides methods to map a portion of RAM dynamically.
pub trait RamMapper: Send + Sync {
    type Err: Error + Send + Sync + 'static;

    unsafe fn map(&self, host: *mut u8, vm: usize, len: NonZero<usize>) -> Result<(), Self::Err>;
}
