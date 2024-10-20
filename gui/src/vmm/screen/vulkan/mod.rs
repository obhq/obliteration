// SPDX-License-Identifier: MIT OR Apache-2.0
use self::buffer::VulkanBuffer;
use self::ffi::{
    create_device, enumerate_device_extension_properties, enumerate_device_layer_properties,
    enumerate_physical_device_groups, enumerate_physical_devices, get_device_proc_addr,
    get_physical_device_external_buffer_properties, get_physical_device_external_fence_properties,
    get_physical_device_external_semaphore_properties, get_physical_device_features,
    get_physical_device_features2, get_physical_device_format_properties,
    get_physical_device_format_properties2, get_physical_device_image_format_properties,
    get_physical_device_image_format_properties2, get_physical_device_memory_properties,
    get_physical_device_memory_properties2, get_physical_device_properties,
    get_physical_device_properties2, get_physical_device_queue_family_properties,
    get_physical_device_queue_family_properties2,
    get_physical_device_sparse_image_format_properties,
    get_physical_device_sparse_image_format_properties2, get_physical_device_tool_properties,
};
use super::{Screen, ScreenBuffer};
use crate::vmm::VmmScreen;
use ash::vk::{DeviceCreateInfo, DeviceQueueCreateInfo, Handle, QueueFlags};
use ash::{Device, Instance, InstanceFnV1_0, InstanceFnV1_1, InstanceFnV1_3};
use std::sync::Arc;
use thiserror::Error;

mod buffer;
mod ffi;

/// Implementation of [`Screen`] using Vulkan.
pub struct Vulkan {
    buffer: Arc<VulkanBuffer>,
    device: Device,
}

impl Vulkan {
    pub fn new(screen: &VmmScreen) -> Result<Self, VulkanError> {
        // Wrap VkInstance.
        let instance = screen.vk_instance.try_into().unwrap();
        let instance = ash::vk::Instance::from_raw(instance);
        let instance = Instance::from_parts_1_3(
            instance,
            InstanceFnV1_0 {
                destroy_instance: Self::destroy_instance,
                enumerate_physical_devices,
                get_physical_device_features,
                get_physical_device_format_properties,
                get_physical_device_image_format_properties,
                get_physical_device_properties,
                get_physical_device_queue_family_properties,
                get_physical_device_memory_properties,
                get_device_proc_addr,
                create_device,
                enumerate_device_extension_properties,
                enumerate_device_layer_properties,
                get_physical_device_sparse_image_format_properties,
            },
            InstanceFnV1_1 {
                enumerate_physical_device_groups,
                get_physical_device_features2,
                get_physical_device_properties2,
                get_physical_device_format_properties2,
                get_physical_device_image_format_properties2,
                get_physical_device_queue_family_properties2,
                get_physical_device_memory_properties2,
                get_physical_device_sparse_image_format_properties2,
                get_physical_device_external_buffer_properties,
                get_physical_device_external_fence_properties,
                get_physical_device_external_semaphore_properties,
            },
            InstanceFnV1_3 {
                get_physical_device_tool_properties,
            },
        );

        // Wrap VkPhysicalDevice.
        let physical = screen.vk_device.try_into().unwrap();
        let physical = ash::vk::PhysicalDevice::from_raw(physical);

        // Setup VkDeviceQueueCreateInfo.
        let queue = unsafe { instance.get_physical_device_queue_family_properties(physical) }
            .iter()
            .position(|p| p.queue_flags.contains(QueueFlags::GRAPHICS))
            .unwrap();
        let queues = [DeviceQueueCreateInfo::default()
            .queue_family_index(queue.try_into().unwrap())
            .queue_priorities(&[1.0])];

        // Create logical device.
        let device = DeviceCreateInfo::default().queue_create_infos(&queues);
        let device = match unsafe { instance.create_device(physical, &device, None) } {
            Ok(v) => v,
            Err(e) => return Err(VulkanError::CreateDeviceFailed(e)),
        };

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
}

/// Implementation of [`Screen::UpdateErr`].
#[derive(Debug, Error)]
pub enum UpdateError {}
