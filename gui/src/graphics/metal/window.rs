use crate::rt::{Hook, RuntimeWindow};
use std::error::Error;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceId, ElementState, InnerSizeWriter, MouseButton, StartCause};
use winit::event_loop::ControlFlow;
use winit::window::WindowId;

/// Implementation of [`RuntimeWindow`] and [`Hook`] for Metal.
pub struct MetalWindow {}

impl RuntimeWindow for MetalWindow {
    fn on_resized(&self, new: PhysicalSize<u32>) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_close_requested(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_focused(&self, gained: bool) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_cursor_moved(
        &self,
        dev: DeviceId,
        pos: PhysicalPosition<f64>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_cursor_left(&self, dev: DeviceId) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_mouse_input(
        &self,
        dev: DeviceId,
        st: ElementState,
        btn: MouseButton,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_scale_factor_changed(
        &self,
        new: f64,
        sw: InnerSizeWriter,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_redraw_requested(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }
}

impl Hook for MetalWindow {
    fn new_events(&self, cause: &StartCause) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn pre_window_event(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn window_destroyed(&self, id: WindowId) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn post_window_event(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn about_to_wait(&self) -> Result<ControlFlow, Box<dyn Error + Send + Sync>> {
        todo!()
    }
}
