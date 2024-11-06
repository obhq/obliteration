// SPDX-License-Identifier: MIT OR Apache-2.0
use self::buffer::VulkanBuffer;
use super::{Screen, ScreenBuffer};
use crate::vmm::VmmScreen;
use ash::vk::{
    ApplicationInfo, DeviceCreateInfo, DeviceQueueCreateInfo, Handle, InstanceCreateInfo,
    QueueFlags,
};
use ash::Device;
use std::sync::Arc;
use thiserror::Error;

mod buffer;

/// Implementation of [`Screen`] using Vulkan.
pub struct Vulkan {
    buffer: Arc<VulkanBuffer>,
    device: Device,
}

impl Vulkan {
    pub fn from_screen(screen: &VmmScreen) -> Result<Self, VulkanError> {
        // Wrap VkInstance.
        let appinfo = ApplicationInfo::default()
            .application_name(c"Obliteration")
            .application_version(0)
            .api_version(ash::vk::make_api_version(0, 1, 0, 0));

        let create_info = InstanceCreateInfo::default().application_info(&appinfo);

        let entry = ash::Entry::linked();

        let instance = unsafe { entry.create_instance(&create_info, None) }
            .map_err(VulkanError::CreateInstanceFailed)?;

        // Wrap VkPhysicalDevice.
        let physical = screen.vk_device.try_into().unwrap();
        let physical = ash::vk::PhysicalDevice::from_raw(physical);

        // Setup VkDeviceQueueCreateInfo.
        let queue = unsafe { instance.get_physical_device_queue_family_properties(physical) }
            .into_iter()
            .position(|p| p.queue_flags.contains(QueueFlags::GRAPHICS))
            .unwrap();

        let queues = [DeviceQueueCreateInfo::default()
            .queue_family_index(queue.try_into().unwrap())
            .queue_priorities(&[1.0])];

        // Create logical device.
        let device = DeviceCreateInfo::default().queue_create_infos(&queues);
        let device = unsafe { instance.create_device(physical, &device, None) }
            .map_err(VulkanError::CreateDeviceFailed)?;

        Ok(Self {
            buffer: Arc::new(VulkanBuffer::new()),
            device,
        })
    }

    unsafe extern "system" fn destroy_instance(
        _: ash::vk::Instance,
        _: *const ash::vk::AllocationCallbacks<'_>,
    ) {
        unimplemented!()
    }
}

impl Drop for Vulkan {
    fn drop(&mut self) {
        unsafe { self.device.device_wait_idle().unwrap() };
        unsafe { self.device.destroy_device(None) };
    }
}

impl Screen for Vulkan {
    type Buffer = VulkanBuffer;
    type UpdateErr = UpdateError;

    fn buffer(&self) -> &Arc<Self::Buffer> {
        &self.buffer
    }

    fn update(&mut self) -> Result<(), Self::UpdateErr> {
        Ok(())
    }
}

/// Represents an error when [`Vulkan::new()`] fails.
#[derive(Debug, Error)]
pub enum VulkanError {
    #[error("couldn't create a logical device")]
    CreateDeviceFailed(#[source] ash::vk::Result),

    #[error("couldn't create a Vulkan instance")]
    CreateInstanceFailed(#[source] ash::vk::Result),
}

/// Implementation of [`Screen::UpdateErr`].
#[derive(Debug, Error)]
pub enum UpdateError {}
