use std::error::Error;

/// Encapsulates winit window with application-specific logic.
pub trait RuntimeWindow {
    fn redraw(&self) -> Result<(), Box<dyn Error>>;
}
