use std::error::Error;
use winit::dpi::PhysicalSize;

/// Encapsulates winit window with application-specific logic.
pub trait RuntimeWindow {
    fn update_size(&self, v: PhysicalSize<u32>) -> Result<(), Box<dyn Error>>;
    fn update_scale_factor(&self, v: f64) -> Result<(), Box<dyn Error>>;
    fn redraw(&self) -> Result<(), Box<dyn Error>>;
}
