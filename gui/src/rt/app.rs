use std::error::Error;

/// Provides application-specific methods for runtime to use.
pub trait App: 'static {
    async fn error(&self, e: impl Error);
}
