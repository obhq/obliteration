use super::engine::Vulkan;
use super::GraphicsError;
use crate::rt::{Hook, RuntimeWindow};
use ash::vk::SurfaceKHR;
use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceId, ElementState, InnerSizeWriter, MouseButton, StartCause};
use winit::event_loop::ControlFlow;
use winit::window::{Window, WindowId};

/// Implementation of [`RuntimeWindow`] and [`Hook`] for Vulkan.
///
/// Fields in this struct must be dropped in a correct order.
pub struct VulkanWindow {
    surface: SurfaceKHR,
    window: Window,
    engine: Arc<Vulkan>,
}

impl VulkanWindow {
    pub fn new(
        engine: &Arc<Vulkan>,
        window: Window,
    ) -> Result<Rc<Self>, Box<dyn Error + Send + Sync>> {
        // Create VkSurfaceKHR.
        let surface =
            unsafe { engine.create_surface(&window) }.map_err(GraphicsError::CreateSurface)?;

        Ok(Rc::new(Self {
            surface,
            window,
            engine: engine.clone(),
        }))
    }
}

impl Drop for VulkanWindow {
    fn drop(&mut self) {
        unsafe { self.engine.destroy_surface(self.surface) };
    }
}

impl RuntimeWindow for VulkanWindow {
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

impl Hook for VulkanWindow {
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
