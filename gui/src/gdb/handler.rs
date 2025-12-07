use std::num::NonZero;

/// Provides methods to handle debug events.
pub trait GdbHandler {
    fn active_thread(&mut self) -> impl IntoIterator<Item = NonZero<usize>>;
}
