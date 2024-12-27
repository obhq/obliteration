// SPDX-License-Identifier: MIT OR Apache-2.0
use super::{GraphicsError, Vulkan};
use crate::graphics::Screen;
use crate::profile::Profile;
use ash::vk::{DeviceCreateInfo, DeviceQueueCreateInfo, QueueFlags, SurfaceKHR};
use ash::Device;
use ash_window::create_surface;
use rwh05::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::window::Window;

/// Implementation of [`Screen`] using Vulkan.
///
/// Fields in this struct must be dropped in a correct order.
pub struct VulkanScreen {
    device: Device,
    glob: Vulkan,
}

impl VulkanScreen {
    pub fn new(glob: Vulkan, profile: &Profile) -> Result<Self, GraphicsError> {
        // TODO: Use selected device.
        let physical = glob.devices.first().unwrap().device;

        // Setup VkDeviceQueueCreateInfo.
        let instance = &glob.instance;
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

        Ok(Self { device, glob })
    }

    /// # Safety
    /// The returned [`SurfaceKHR`] must be destroyed before `win` and this [`VulkanScreen`].
    pub unsafe fn create_surface(&self, win: &Window) -> Result<SurfaceKHR, ash::vk::Result> {
        let dh = win.raw_display_handle();
        let wh = win.raw_window_handle();

        create_surface(&self.glob.entry, &self.glob.instance, dh, wh, None)
    }

    /// # Safety
    /// See `vkDestroySurfaceKHR` docs for valid usage.
    pub unsafe fn destroy_surface(&self, surface: SurfaceKHR) {
        self.glob.surface.destroy_surface(surface, None);
    }
}

impl Drop for VulkanScreen {
    fn drop(&mut self) {
        // Free device.
        unsafe { self.device.device_wait_idle().unwrap() };
        unsafe { self.device.destroy_device(None) };
    }
}

impl Screen for VulkanScreen {}
