// SPDX-License-Identifier: MIT OR Apache-2.0
use super::{GraphicsError, VulkanBuilder};
use crate::graphics::Graphics;
use crate::profile::Profile;
use ash::vk::{DeviceCreateInfo, DeviceQueueCreateInfo, QueueFlags, SurfaceKHR};
use ash::Device;
use ash_window::create_surface;
use rwh05::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::window::Window;

/// Implementation of [`Graphics`] using Vulkan.
///
/// Fields in this struct must be dropped in a correct order.
pub struct Vulkan {
    device: Device,
    builder: VulkanBuilder,
}

impl Vulkan {
    pub fn new(b: VulkanBuilder, profile: &Profile) -> Result<Self, GraphicsError> {
        // TODO: Use selected device.
        let physical = b.devices.first().unwrap().device;

        // Setup VkDeviceQueueCreateInfo.
        let instance = &b.instance;
        let queue = unsafe { instance.get_physical_device_queue_family_properties(physical) }
            .into_iter()
            .position(|p| p.queue_flags.contains(QueueFlags::GRAPHICS))
            .unwrap(); // We required all selectable devices to supports graphics operations.
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
        let device = unsafe { instance.create_device(physical, &device, None) }
            .map_err(GraphicsError::CreateDevice)?;

        Ok(Self { device, builder: b })
    }

    /// # Safety
    /// The returned [`SurfaceKHR`] must be destroyed before `win` and this [`Vulkan`].
    pub unsafe fn create_surface(&self, win: &Window) -> Result<SurfaceKHR, ash::vk::Result> {
        let dh = win.raw_display_handle();
        let wh = win.raw_window_handle();

        create_surface(&self.builder.entry, &self.builder.instance, dh, wh, None)
    }

    /// # Safety
    /// See `vkDestroySurfaceKHR` docs for valid usage.
    pub unsafe fn destroy_surface(&self, surface: SurfaceKHR) {
        self.builder.surface.destroy_surface(surface, None);
    }
}

impl Drop for Vulkan {
    fn drop(&mut self) {
        // Free device.
        unsafe { self.device.device_wait_idle().unwrap() };
        unsafe { self.device.destroy_device(None) };
    }
}

impl Graphics for Vulkan {}
