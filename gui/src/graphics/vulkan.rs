// SPDX-License-Identifier: MIT OR Apache-2.0
use ash::vk::{ApplicationInfo, InstanceCreateInfo};
use std::ffi::CStr;
use thiserror::Error;

pub struct Vulkan {
    entry: ash::Entry,
    instance: ash::Instance,
    devices: Vec<VulkanPhysicalDevice>,
}

impl super::GraphicsApi for Vulkan {
    type PhysicalDevice = VulkanPhysicalDevice;

    type InitError = VulkanInitError;

    fn init() -> Result<Self, Self::InitError> {
        let entry = ash::Entry::linked();

        let app_info = ApplicationInfo::default().application_name(c"Obliteration");

        let create_info = InstanceCreateInfo::default().application_info(&app_info);

        let instance = unsafe { entry.create_instance(&create_info, None) }
            .map_err(VulkanInitError::CreateInstanceFailed)?;

        let devices = unsafe { instance.enumerate_physical_devices() }
            .map_err(VulkanInitError::EnumeratePhysicalDevicesFailed)?
            .into_iter()
            .map(|device| -> Result<VulkanPhysicalDevice, VulkanInitError> {
                let properties = unsafe { instance.get_physical_device_properties(device) };

                let name = CStr::from_bytes_until_nul(unsafe {
                    std::slice::from_raw_parts(properties.device_name.as_ptr().cast(), 256)
                })
                .map_err(|_| VulkanInitError::DeviceNameInvalid)?
                .to_str()
                .map_err(VulkanInitError::DeviceNameInvalidUtf8)?
                .to_owned();

                Ok(VulkanPhysicalDevice { device, name })
            })
            .collect::<Result<_, VulkanInitError>>()?;

        Ok(Self {
            entry,
            instance,
            devices,
        })
    }

    fn enumerate_physical_devices(&self) -> &[Self::PhysicalDevice] {
        &self.devices
    }
}

impl Drop for Vulkan {
    fn drop(&mut self) {
        unsafe { self.instance.destroy_instance(None) };
    }
}

pub struct VulkanPhysicalDevice {
    device: ash::vk::PhysicalDevice,
    name: String,
}

impl super::PhysicalDevice for VulkanPhysicalDevice {
    fn name(&self) -> &str {
        &self.name
    }
}

/// Represents an error when [`Vulkan::init()`] fails.
#[derive(Debug, Error)]
pub enum VulkanInitError {
    #[error("couldn't create Vulkan instance")]
    CreateInstanceFailed(#[source] ash::vk::Result),

    #[error("couldn't enumerate physical devices")]
    EnumeratePhysicalDevicesFailed(#[source] ash::vk::Result),

    #[error("no null byte in device name")]
    DeviceNameInvalid,

    #[error("device name is not valid UTF-8")]
    DeviceNameInvalidUtf8(#[source] std::str::Utf8Error),
}
