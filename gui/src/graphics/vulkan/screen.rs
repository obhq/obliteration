// SPDX-License-Identifier: MIT OR Apache-2.0
use super::buffer::VulkanBuffer;
use crate::graphics::Screen;
use crate::vmm::VmmScreen;
use ash::vk::{DeviceCreateInfo, DeviceQueueCreateInfo, Handle, QueueFlags};
use ash::Device;
use std::sync::Arc;
use thiserror::Error;

/// Implementation of [`Screen`] using Vulkan.
pub struct VulkanScreen {
    buffer: Arc<VulkanBuffer>,
    device: Device,
}

impl VulkanScreen {
    pub fn new() -> Result<Self, VulkanScreenError> {
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

        Ok(Self {
            buffer: Arc::new(VulkanBuffer::new()),
            device,
        })
    }
}

impl Drop for VulkanScreen {
    fn drop(&mut self) {
        unsafe { self.device.device_wait_idle().unwrap() };
        unsafe { self.device.destroy_device(None) };
    }
}

impl Screen for VulkanScreen {
    type Buffer = VulkanBuffer;
    type RunErr = RunError;

    fn buffer(&self) -> &Arc<Self::Buffer> {
        &self.buffer
    }

    fn run(self) -> Result<(), Self::RunErr> {
        todo!()
    }
}

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

/// Implementation of [`Screen::RunErr`].
#[derive(Debug, Error)]
pub enum RunError {}
