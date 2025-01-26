use std::error::Error;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceId, ElementState, InnerSizeWriter, MouseButton};
use winit::window::WindowId;

/// Encapsulates winit window with window-specific logic.
///
/// The event loop will exit immediately if any method return an error.
pub trait WindowHandler: WinitWindow {
    fn on_resized(&self, new: PhysicalSize<u32>) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn on_close_requested(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn on_focused(&self, gained: bool) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn on_cursor_moved(
        &self,
        dev: DeviceId,
        pos: PhysicalPosition<f64>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn on_cursor_left(&self, dev: DeviceId) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn on_mouse_input(
        &self,
        dev: DeviceId,
        st: ElementState,
        btn: MouseButton,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn on_scale_factor_changed(
        &self,
        new: f64,
        sw: InnerSizeWriter,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn on_redraw_requested(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
}

/// Provides method to return winit properties.
pub trait WinitWindow {
    fn id(&self) -> WindowId;
}
