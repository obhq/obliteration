use super::engine::Vulkan;
use super::GraphicsError;
use crate::rt::{Hook, RuntimeWindow};
use ash::vk::SurfaceKHR;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};
use std::error::Error;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
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
    shutdown: Arc<AtomicBool>,
}

impl VulkanWindow {
    pub fn new(
        engine: &Arc<Vulkan>,
        window: Window,
        shutdown: &Arc<AtomicBool>,
    ) -> Result<Rc<Self>, Box<dyn Error + Send + Sync>> {
        // Create VkSurfaceKHR.
        let surface =
            unsafe { engine.create_surface(&window) }.map_err(GraphicsError::CreateSurface)?;

        Ok(Rc::new(Self {
            surface,
            window,
            engine: engine.clone(),
            shutdown: shutdown.clone(),
        }))
    }
}

impl Drop for VulkanWindow {
    fn drop(&mut self) {
        unsafe { self.engine.destroy_surface(self.surface) };
    }
}

impl RuntimeWindow for VulkanWindow {
    fn id(&self) -> WindowId {
        self.window.id()
    }

    fn on_resized(&self, _: PhysicalSize<u32>) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Vulkan windows does not allowed to resize.
        Ok(())
    }

    fn on_close_requested(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.shutdown.store(true, Ordering::Relaxed);
        Ok(())
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
        _: f64,
        _: InnerSizeWriter,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    fn on_redraw_requested(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.window.request_redraw();
        Ok(())
    }
}

impl HasDisplayHandle for VulkanWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.window.display_handle()
    }
}

impl HasWindowHandle for VulkanWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.window.window_handle()
    }
}

impl Hook for VulkanWindow {
    fn new_events(&self, _: &StartCause) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    fn pre_window_event(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    fn window_destroyed(&self, _: WindowId) -> Result<(), Box<dyn Error + Send + Sync>> {
        // This never be our window since we live forever until the event loop exit.
        Ok(())
    }

    fn post_window_event(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    fn about_to_wait(&self) -> Result<ControlFlow, Box<dyn Error + Send + Sync>> {
        // TODO: Not sure if we need Poll here since we always request a redraw when we received
        // redraw requested.
        Ok(ControlFlow::Wait)
    }
}
