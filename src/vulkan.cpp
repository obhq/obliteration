#include "vulkan.hpp"

#include <QVulkanFunctions>

QVulkanFunctions *vkFunctions;

extern "C" VkResult vmm_vk_enumerate_physical_devices(
    VkInstance instance,
    uint32_t *p_physical_device_count,
    VkPhysicalDevice *p_physical_devices)
{
    return vkFunctions->vkEnumeratePhysicalDevices(
        instance,
        p_physical_device_count,
        p_physical_devices);
}

extern "C" void vmm_vk_get_physical_device_features(
    VkPhysicalDevice physical_device,
    VkPhysicalDeviceFeatures *p_features)
{
    vkFunctions->vkGetPhysicalDeviceFeatures(physical_device, p_features);
}

extern "C" void vmm_vk_get_physical_device_format_properties(
    VkPhysicalDevice physical_device,
    VkFormat format,
    VkFormatProperties *p_format_properties)
{
    vkFunctions->vkGetPhysicalDeviceFormatProperties(physical_device, format, p_format_properties);
}

extern "C" VkResult vmm_vk_get_physical_device_image_format_properties(
    VkPhysicalDevice physical_device,
    VkFormat format,
    VkImageType ty,
    VkImageTiling tiling,
    VkImageUsageFlags usage,
    VkImageCreateFlags flags,
    VkImageFormatProperties *p_image_format_properties)
{
    return vkFunctions->vkGetPhysicalDeviceImageFormatProperties(
        physical_device,
        format,
        ty,
        tiling,
        usage,
        flags,
        p_image_format_properties);
}

extern "C" void vmm_vk_get_physical_device_properties(
    VkPhysicalDevice physical_device,
    VkPhysicalDeviceProperties *p_properties)
{
    vkFunctions->vkGetPhysicalDeviceProperties(physical_device, p_properties);
}

extern "C" void vmm_vk_get_physical_device_queue_family_properties(
    VkPhysicalDevice physical_device,
    uint32_t *p_queue_family_property_count,
    VkQueueFamilyProperties *p_queue_family_properties)
{
    vkFunctions->vkGetPhysicalDeviceQueueFamilyProperties(
        physical_device,
        p_queue_family_property_count,
        p_queue_family_properties);
}

extern "C" void vmm_vk_get_physical_device_memory_properties(
    VkPhysicalDevice physical_device,
    VkPhysicalDeviceMemoryProperties *p_memory_properties)
{
    vkFunctions->vkGetPhysicalDeviceMemoryProperties(physical_device, p_memory_properties);
}

extern "C" PFN_vkVoidFunction vmm_vk_get_device_proc_addr(VkDevice device, const char *p_name)
{
    return vkFunctions->vkGetDeviceProcAddr(device, p_name);
}

extern "C" VkResult vmm_vk_create_device(
    VkPhysicalDevice physical_device,
    const VkDeviceCreateInfo *p_create_info,
    const VkAllocationCallbacks *p_allocator,
    VkDevice *p_device)
{
    return vkFunctions->vkCreateDevice(physical_device, p_create_info, p_allocator, p_device);
}

extern "C" VkResult vmm_vk_enumerate_device_extension_properties(
    VkPhysicalDevice physical_device,
    const char *p_layer_name,
    uint32_t *p_property_count,
    VkExtensionProperties *p_properties)
{
    return vkFunctions->vkEnumerateDeviceExtensionProperties(
        physical_device,
        p_layer_name,
        p_property_count,
        p_properties);
}

extern "C" VkResult vmm_vk_enumerate_device_layer_properties(
    VkPhysicalDevice physical_device,
    uint32_t *p_property_count,
    VkLayerProperties *p_properties)
{
    return vkFunctions->vkEnumerateDeviceLayerProperties(
        physical_device,
        p_property_count,
        p_properties);
}

extern "C" void vmm_vk_get_physical_device_sparse_image_format_properties(
    VkPhysicalDevice physical_device,
    VkFormat format,
    VkImageType ty,
    VkSampleCountFlagBits samples,
    VkImageUsageFlags usage,
    VkImageTiling tiling,
    uint32_t *p_property_count,
    VkSparseImageFormatProperties *p_properties)
{
    vkFunctions->vkGetPhysicalDeviceSparseImageFormatProperties(
        physical_device,
        format,
        ty,
        samples,
        usage,
        tiling,
        p_property_count,
        p_properties);
}

extern "C" VkResult vmm_vk_enumerate_physical_device_groups(
    VkInstance instance,
    uint32_t *p_physical_device_group_count,
    VkPhysicalDeviceGroupProperties *p_physical_device_group_properties)
{
    return vkFunctions->vkEnumeratePhysicalDeviceGroups(
        instance,
        p_physical_device_group_count,
        p_physical_device_group_properties);
}

extern "C" void vmm_vk_get_physical_device_features2(
    VkPhysicalDevice physical_device,
    VkPhysicalDeviceFeatures2 *p_features)
{
    vkFunctions->vkGetPhysicalDeviceFeatures2(physical_device, p_features);
}

extern "C" void vmm_vk_get_physical_device_properties2(
    VkPhysicalDevice physical_device,
    VkPhysicalDeviceProperties2 *p_properties)
{
    vkFunctions->vkGetPhysicalDeviceProperties2(physical_device, p_properties);
}

extern "C" void vmm_vk_get_physical_device_format_properties2(
    VkPhysicalDevice physical_device,
    VkFormat format,
    VkFormatProperties2 *p_format_properties)
{
    vkFunctions->vkGetPhysicalDeviceFormatProperties2(physical_device, format, p_format_properties);
}

extern "C" VkResult vmm_vk_get_physical_device_image_format_properties2(
    VkPhysicalDevice physical_device,
    const VkPhysicalDeviceImageFormatInfo2 *p_image_format_info,
    VkImageFormatProperties2 *p_image_format_properties)
{
    return vkFunctions->vkGetPhysicalDeviceImageFormatProperties2(
        physical_device,
        p_image_format_info,
        p_image_format_properties);
}

extern "C" void vmm_vk_get_physical_device_queue_family_properties2(
    VkPhysicalDevice physical_device,
    uint32_t *p_queue_family_property_count,
    VkQueueFamilyProperties2 *p_queue_family_properties)
{
    vkFunctions->vkGetPhysicalDeviceQueueFamilyProperties2(
        physical_device,
        p_queue_family_property_count,
        p_queue_family_properties);
}

extern "C" void vmm_vk_get_physical_device_memory_properties2(
    VkPhysicalDevice physical_device,
    VkPhysicalDeviceMemoryProperties2 *p_memory_properties)
{
    vkFunctions->vkGetPhysicalDeviceMemoryProperties2(physical_device, p_memory_properties);
}

extern "C" void vmm_vk_get_physical_device_sparse_image_format_properties2(
    VkPhysicalDevice physical_device,
    const VkPhysicalDeviceSparseImageFormatInfo2 *p_format_info,
    uint32_t *p_property_count,
    VkSparseImageFormatProperties2 *p_properties)
{
    vkFunctions->vkGetPhysicalDeviceSparseImageFormatProperties2(
        physical_device,
        p_format_info,
        p_property_count,
        p_properties);
}

extern "C" void vmm_vk_get_physical_device_external_buffer_properties(
    VkPhysicalDevice physical_device,
    const VkPhysicalDeviceExternalBufferInfo *p_external_buffer_info,
    VkExternalBufferProperties *p_external_buffer_properties)
{
    vkFunctions->vkGetPhysicalDeviceExternalBufferProperties(
        physical_device,
        p_external_buffer_info,
        p_external_buffer_properties);
}

extern "C" void vmm_vk_get_physical_device_external_fence_properties(
    VkPhysicalDevice physical_device,
    const VkPhysicalDeviceExternalFenceInfo *p_external_fence_info,
    VkExternalFenceProperties *p_external_fence_properties)
{
    vkFunctions->vkGetPhysicalDeviceExternalFenceProperties(
        physical_device,
        p_external_fence_info,
        p_external_fence_properties);
}

extern "C" void vmm_vk_get_physical_device_external_semaphore_properties(
    VkPhysicalDevice physical_device,
    const VkPhysicalDeviceExternalSemaphoreInfo *p_external_semaphore_info,
    VkExternalSemaphoreProperties *p_external_semaphore_properties)
{
    vkFunctions->vkGetPhysicalDeviceExternalSemaphoreProperties(
        physical_device,
        p_external_semaphore_info,
        p_external_semaphore_properties);
}

extern "C" VkResult vmm_vk_get_physical_device_tool_properties(
    VkPhysicalDevice physical_device,
    uint32_t *p_tool_count,
    VkPhysicalDeviceToolProperties *p_tool_properties)
{
    return vkFunctions->vkGetPhysicalDeviceToolProperties(
        physical_device,
        p_tool_count,
        p_tool_properties);
}
