use ash::vk::{
    AllocationCallbacks, Device, DeviceCreateInfo, ExtensionProperties, ExternalBufferProperties,
    ExternalFenceProperties, ExternalSemaphoreProperties, Format, FormatProperties,
    FormatProperties2, ImageCreateFlags, ImageFormatProperties, ImageFormatProperties2,
    ImageTiling, ImageType, ImageUsageFlags, Instance, LayerProperties, PFN_vkVoidFunction,
    PhysicalDevice, PhysicalDeviceExternalBufferInfo, PhysicalDeviceExternalFenceInfo,
    PhysicalDeviceExternalSemaphoreInfo, PhysicalDeviceFeatures, PhysicalDeviceFeatures2,
    PhysicalDeviceGroupProperties, PhysicalDeviceImageFormatInfo2, PhysicalDeviceMemoryProperties,
    PhysicalDeviceMemoryProperties2, PhysicalDeviceProperties, PhysicalDeviceProperties2,
    PhysicalDeviceSparseImageFormatInfo2, PhysicalDeviceToolProperties, QueueFamilyProperties,
    QueueFamilyProperties2, Result, SampleCountFlags, SparseImageFormatProperties,
    SparseImageFormatProperties2,
};
use std::ffi::c_char;

extern "system" {
    #[link_name = "vmm_vk_create_device"]
    pub fn create_device(
        physical_device: PhysicalDevice,
        p_create_info: *const DeviceCreateInfo<'_>,
        p_allocator: *const AllocationCallbacks<'_>,
        p_device: *mut Device,
    ) -> Result;

    #[link_name = "vmm_vk_enumerate_device_extension_properties"]
    pub fn enumerate_device_extension_properties(
        physical_device: PhysicalDevice,
        p_layer_name: *const c_char,
        p_property_count: *mut u32,
        p_properties: *mut ExtensionProperties,
    ) -> Result;

    #[link_name = "vmm_vk_enumerate_device_layer_properties"]
    pub fn enumerate_device_layer_properties(
        physical_device: PhysicalDevice,
        p_property_count: *mut u32,
        p_properties: *mut LayerProperties,
    ) -> Result;

    #[link_name = "vmm_vk_enumerate_physical_device_groups"]
    pub fn enumerate_physical_device_groups(
        instance: Instance,
        p_physical_device_group_count: *mut u32,
        p_physical_device_group_properties: *mut PhysicalDeviceGroupProperties<'_>,
    ) -> Result;

    #[link_name = "vmm_vk_enumerate_physical_devices"]
    pub fn enumerate_physical_devices(
        instance: Instance,
        p_physical_device_count: *mut u32,
        p_physical_devices: *mut PhysicalDevice,
    ) -> Result;

    #[link_name = "vmm_vk_get_device_proc_addr"]
    pub fn get_device_proc_addr(device: Device, p_name: *const c_char) -> PFN_vkVoidFunction;

    #[link_name = "vmm_vk_get_physical_device_external_buffer_properties"]
    pub fn get_physical_device_external_buffer_properties(
        physical_device: PhysicalDevice,
        p_external_buffer_info: *const PhysicalDeviceExternalBufferInfo<'_>,
        p_external_buffer_properties: *mut ExternalBufferProperties<'_>,
    );

    #[link_name = "vmm_vk_get_physical_device_external_fence_properties"]
    pub fn get_physical_device_external_fence_properties(
        physical_device: PhysicalDevice,
        p_external_fence_info: *const PhysicalDeviceExternalFenceInfo<'_>,
        p_external_fence_properties: *mut ExternalFenceProperties<'_>,
    );

    #[link_name = "vmm_vk_get_physical_device_external_semaphore_properties"]
    pub fn get_physical_device_external_semaphore_properties(
        physical_device: PhysicalDevice,
        p_external_semaphore_info: *const PhysicalDeviceExternalSemaphoreInfo<'_>,
        p_external_semaphore_properties: *mut ExternalSemaphoreProperties<'_>,
    );

    #[link_name = "vmm_vk_get_physical_device_features"]
    pub fn get_physical_device_features(
        physical_device: PhysicalDevice,
        p_features: *mut PhysicalDeviceFeatures,
    );

    #[link_name = "vmm_vk_get_physical_device_features2"]
    pub fn get_physical_device_features2(
        physical_device: PhysicalDevice,
        p_features: *mut PhysicalDeviceFeatures2<'_>,
    );

    #[link_name = "vmm_vk_get_physical_device_format_properties"]
    pub fn get_physical_device_format_properties(
        physical_device: PhysicalDevice,
        format: Format,
        p_format_properties: *mut FormatProperties,
    );

    #[link_name = "vmm_vk_get_physical_device_format_properties2"]
    pub fn get_physical_device_format_properties2(
        physical_device: PhysicalDevice,
        format: Format,
        p_format_properties: *mut FormatProperties2<'_>,
    );

    #[link_name = "vmm_vk_get_physical_device_image_format_properties"]
    pub fn get_physical_device_image_format_properties(
        physical_device: PhysicalDevice,
        format: Format,
        ty: ImageType,
        tiling: ImageTiling,
        usage: ImageUsageFlags,
        flags: ImageCreateFlags,
        p_image_format_properties: *mut ImageFormatProperties,
    ) -> Result;

    #[link_name = "vmm_vk_get_physical_device_image_format_properties2"]
    pub fn get_physical_device_image_format_properties2(
        physical_device: PhysicalDevice,
        p_image_format_info: *const PhysicalDeviceImageFormatInfo2<'_>,
        p_image_format_properties: *mut ImageFormatProperties2<'_>,
    ) -> Result;

    #[link_name = "vmm_vk_get_physical_device_memory_properties"]
    pub fn get_physical_device_memory_properties(
        physical_device: PhysicalDevice,
        p_memory_properties: *mut PhysicalDeviceMemoryProperties,
    );

    #[link_name = "vmm_vk_get_physical_device_memory_properties2"]
    pub fn get_physical_device_memory_properties2(
        physical_device: PhysicalDevice,
        p_memory_properties: *mut PhysicalDeviceMemoryProperties2<'_>,
    );

    #[link_name = "vmm_vk_get_physical_device_properties"]
    pub fn get_physical_device_properties(
        physical_device: PhysicalDevice,
        p_properties: *mut PhysicalDeviceProperties,
    );

    #[link_name = "vmm_vk_get_physical_device_properties2"]
    pub fn get_physical_device_properties2(
        physical_device: PhysicalDevice,
        p_properties: *mut PhysicalDeviceProperties2<'_>,
    );

    #[link_name = "vmm_vk_get_physical_device_queue_family_properties"]
    pub fn get_physical_device_queue_family_properties(
        physical_device: PhysicalDevice,
        p_queue_family_property_count: *mut u32,
        p_queue_family_properties: *mut QueueFamilyProperties,
    );

    #[link_name = "vmm_vk_get_physical_device_queue_family_properties2"]
    pub fn get_physical_device_queue_family_properties2(
        physical_device: PhysicalDevice,
        p_queue_family_property_count: *mut u32,
        p_queue_family_properties: *mut QueueFamilyProperties2<'_>,
    );

    #[link_name = "vmm_vk_get_physical_device_sparse_image_format_properties"]
    pub fn get_physical_device_sparse_image_format_properties(
        physical_device: PhysicalDevice,
        format: Format,
        ty: ImageType,
        samples: SampleCountFlags,
        usage: ImageUsageFlags,
        tiling: ImageTiling,
        p_property_count: *mut u32,
        p_properties: *mut SparseImageFormatProperties,
    );

    #[link_name = "vmm_vk_get_physical_device_sparse_image_format_properties2"]
    pub fn get_physical_device_sparse_image_format_properties2(
        physical_device: PhysicalDevice,
        p_format_info: *const PhysicalDeviceSparseImageFormatInfo2<'_>,
        p_property_count: *mut u32,
        p_properties: *mut SparseImageFormatProperties2<'_>,
    );

    #[link_name = "vmm_vk_get_physical_device_tool_properties"]
    pub fn get_physical_device_tool_properties(
        physical_device: PhysicalDevice,
        p_tool_count: *mut u32,
        p_tool_properties: *mut PhysicalDeviceToolProperties<'_>,
    ) -> Result;
}
