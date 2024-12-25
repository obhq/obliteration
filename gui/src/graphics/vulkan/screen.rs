// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::graphics::Screen;
use crate::profile::Profile;
use crate::rt::{Hook, RuntimeWindow};
use crate::vmm::VmmScreen;
use ash::vk::{DeviceCreateInfo, DeviceQueueCreateInfo, Handle, QueueFlags};
use ash::Device;
use std::error::Error;
use std::rc::Rc;
use thiserror::Error;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceId, ElementState, InnerSizeWriter, MouseButton, StartCause};
use winit::event_loop::ControlFlow;
use winit::window::{Window, WindowId};

/// Implementation of [`Screen`] using Vulkan.
pub struct VulkanScreen {
    device: Device,
}

impl VulkanScreen {
    pub fn new(profile: &Profile, win: Window) -> Result<Rc<Self>, Box<dyn Error + Send + Sync>> {
        todo!()
    }

    pub fn from_screen(screen: &VmmScreen) -> Result<Self, VulkanScreenError> {
        let entry = ash::Entry::linked();

        let instance = unsafe {
            ash::Instance::load(
                entry.static_fn(),
                ash::vk::Instance::from_raw(screen.vk_instance.try_into().unwrap()),
            )
        };

        // Wrap VkPhysicalDevice.
        let physical = screen.vk_device.try_into().unwrap();
        let physical = ash::vk::PhysicalDevice::from_raw(physical);

        // Setup VkDeviceQueueCreateInfo.
        let queue = unsafe { instance.get_physical_device_queue_family_properties(physical) }
            .into_iter()
            .position(|p| p.queue_flags.contains(QueueFlags::GRAPHICS))
            .ok_or(VulkanScreenError::NoQueue)?;
        let queue = queue
            .try_into()
            .map_err(|_| VulkanScreenError::QueueOutOfBounds(queue))?;
        let mut queues = DeviceQueueCreateInfo::default();
        let priorities = [1.0];

        queues.queue_family_index = queue;
        queues.queue_count = 1;
        queues.p_queue_priorities = priorities.as_ptr();

        // Setup VkDeviceCreateInfo.
        let mut device = DeviceCreateInfo::default();

        device.p_queue_create_infos = &queues;
        device.queue_create_info_count = 1;

        // Create logical device.
        let device = unsafe { instance.create_device(physical, &device, None) }
            .map_err(VulkanScreenError::CreateDeviceFailed)?;

        Ok(Self { device })
    }
}

impl Drop for VulkanScreen {
    fn drop(&mut self) {
        unsafe { self.device.device_wait_idle().unwrap() };
        unsafe { self.device.destroy_device(None) };
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

/// Represents an error when [`VulkanScreen::new()`] fails.
#[derive(Debug, Error)]
pub enum VulkanScreenError {
    #[error("couldn't find suitable queue")]
    NoQueue,

    #[error("queue index #{0} out of bounds")]
    QueueOutOfBounds(usize),

    #[error("couldn't create a logical device")]
    CreateDeviceFailed(#[source] ash::vk::Result),
}
