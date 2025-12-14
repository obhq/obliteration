use std::num::NonZero;

/// Provides methods to handle debug events.
pub trait GdbHandler {
    type Err: std::error::Error;

    fn active_threads(&mut self) -> impl IntoIterator<Item = NonZero<usize>>;
    async fn suspend_threads(&mut self) -> Result<(), Self::Err>;
}
