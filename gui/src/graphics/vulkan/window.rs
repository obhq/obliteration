use super::GraphicsError;
use super::engine::Vulkan;
use crate::ui::DesktopWindow;
use ash::vk::SurfaceKHR;
use raw_window_handle::HasWindowHandle;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use wae::{Hook, WindowHandler, WinitWindow};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceId, ElementState, InnerSizeWriter, MouseButton, StartCause};
use winit::event_loop::ControlFlow;
use winit::window::{Window, WindowId};

/// Implementation of [`WindowHandler`] and [`Hook`] for Vulkan.
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
    ) -> Result<Self, GraphicsError> {
        // Create VkSurfaceKHR.
        let surface =
            unsafe { engine.create_surface(&window) }.map_err(GraphicsError::CreateSurface)?;

        Ok(Self {
            surface,
            window,
            engine: engine.clone(),
            shutdown: shutdown.clone(),
        })
    }
}

impl Drop for VulkanWindow {
    fn drop(&mut self) {
        unsafe { self.engine.destroy_surface(self.surface) };
    }
}

impl WinitWindow for VulkanWindow {
    fn id(&self) -> WindowId {
        self.window.id()
    }
}

impl DesktopWindow for VulkanWindow {
    fn handle(&self) -> impl HasWindowHandle + '_ {
        &self.window
    }

    #[cfg(target_os = "linux")]
    fn xdg_toplevel(&self) -> Option<std::ptr::NonNull<std::ffi::c_void>> {
        use winit::platform::wayland::WindowExtWayland;

        self.window.xdg_toplevel()
    }
}

impl WindowHandler for VulkanWindow {
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
