use std::error::Error;

/// Encapsulates winit window with application-specific logic.
pub trait RuntimeWindow {
    fn update_scale_factor(&self, v: f64) -> Result<(), Box<dyn Error>>;
    fn redraw(&self) -> Result<(), Box<dyn Error>>;
}
