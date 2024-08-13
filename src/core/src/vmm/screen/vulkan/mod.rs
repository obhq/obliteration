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
use super::{Screen, ScreenBuffer, VmmError};
use crate::vmm::VmmScreen;
use ash::vk::Handle;
use ash::{Instance, InstanceFnV1_0, InstanceFnV1_1, InstanceFnV1_3};
use std::sync::Arc;
use thiserror::Error;

mod buffer;
mod ffi;

/// Implementation of [`Screen`] using Vulkan.
pub struct Vulkan {
    buffer: Arc<VulkanBuffer>,
    instance: Instance,
}

impl Vulkan {
    pub fn new(screen: *const VmmScreen) -> Result<Self, VmmError> {
        // Wrap VkInstance.
        let instance = unsafe { (*screen).vk_instance.try_into().unwrap() };
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

        Ok(Self {
            buffer: Arc::new(VulkanBuffer::new()),
            instance,
        })
    }

    unsafe extern "system" fn destroy_instance(
        _: ash::vk::Instance,
        _: *const ash::vk::AllocationCallbacks<'_>,
    ) {
        unreachable!();
    }
}

impl Screen for Vulkan {
    type Buffer = VulkanBuffer;
    type UpdateErr = UpdateError;

    fn buffer(&self) -> &Arc<Self::Buffer> {
        &self.buffer
    }

    fn update(&mut self) -> Result<(), Self::UpdateErr> {
        todo!()
    }
}

/// Implementation of [`Screen::UpdateErr`].
#[derive(Debug, Error)]
pub enum UpdateError {}
