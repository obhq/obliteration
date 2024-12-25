// SPDX-License-Identifier: MIT OR Apache-2.0
use super::{GraphicsError, Vulkan};
use crate::graphics::Screen;
use crate::profile::Profile;
use crate::rt::{Hook, RuntimeWindow};
use ash::vk::{DeviceCreateInfo, DeviceQueueCreateInfo, QueueFlags, SurfaceKHR};
use ash::Device;
use ash_window::create_surface;
use rwh05::{HasRawDisplayHandle, HasRawWindowHandle};
use std::error::Error;
use std::rc::Rc;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceId, ElementState, InnerSizeWriter, MouseButton, StartCause};
use winit::event_loop::ControlFlow;
use winit::window::{Window, WindowId};

/// Implementation of [`Screen`] using Vulkan.
///
/// Fields in this struct must be dropped in a correct order.
pub struct VulkanScreen {
    device: Device,
    surface: SurfaceKHR,
    glob: Vulkan,
    window: Window,
}

impl VulkanScreen {
    pub fn new(
        glob: Vulkan,
        profile: &Profile,
        window: Window,
    ) -> Result<Rc<Self>, Box<dyn Error + Send + Sync>> {
        // TODO: Use selected device.
        let physical = glob.devices.first().unwrap().device;

        // Setup VkDeviceQueueCreateInfo.
        let instance = &glob.instance;
        let queue = unsafe { instance.get_physical_device_queue_family_properties(physical) }
            .into_iter()
            .position(|p| p.queue_flags.contains(QueueFlags::GRAPHICS))
            .ok_or(GraphicsError::NoQueue)?;
        let mut queues = DeviceQueueCreateInfo::default();
        let priorities = [1.0];

        queues.queue_family_index = queue.try_into().unwrap();
        queues.queue_count = 1;
        queues.p_queue_priorities = priorities.as_ptr();

        // Setup VkDeviceCreateInfo.
        let mut device = DeviceCreateInfo::default();

        device.p_queue_create_infos = &queues;
        device.queue_create_info_count = 1;

        // Create logical device.
        let mut s = Self {
            device: unsafe { instance.create_device(physical, &device, None) }
                .map_err(GraphicsError::CreateDevice)?,
            surface: SurfaceKHR::null(),
            glob,
            window,
        };

        // Create VkSurfaceKHR.
        let dh = s.window.raw_display_handle();
        let wh = s.window.raw_window_handle();

        s.surface = unsafe { create_surface(&s.glob.entry, &s.glob.instance, dh, wh, None) }
            .map_err(GraphicsError::CreateSurface)?;

        Ok(Rc::new(s))
    }
}

impl Drop for VulkanScreen {
    fn drop(&mut self) {
        // Free device.
        unsafe { self.device.device_wait_idle().unwrap() };
        unsafe { self.device.destroy_device(None) };

        // Free surface.
        if self.surface != SurfaceKHR::null() {
            unsafe { self.glob.surface.destroy_surface(self.surface, None) };
        }
    }
}

impl RuntimeWindow for VulkanScreen {
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

impl Hook for VulkanScreen {
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

impl Screen for VulkanScreen {}
