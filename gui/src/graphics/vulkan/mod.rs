// SPDX-License-Identifier: MIT OR Apache-2.0
use self::screen::VulkanScreen;
use super::Graphics;
use ash::vk::{ApplicationInfo, InstanceCreateInfo};
use std::ffi::CStr;
use thiserror::Error;

mod buffer;
mod screen;

pub struct Vulkan {
    entry: ash::Entry,
    instance: ash::Instance,
    devices: Vec<VulkanPhysicalDevice>,
}

impl Graphics for Vulkan {
    type Err = VulkanError;
    type PhysicalDevice = VulkanPhysicalDevice;
    type Screen = VulkanScreen;

    fn new() -> Result<Self, Self::Err> {
        let entry = ash::Entry::linked();

        let app_info = ApplicationInfo::default().application_name(c"Obliteration");

        let create_info = InstanceCreateInfo::default().application_info(&app_info);

        let instance = unsafe { entry.create_instance(&create_info, None) }
            .map_err(VulkanError::CreateInstanceFailed)?;

        let devices = unsafe { instance.enumerate_physical_devices() }
            .map_err(VulkanError::EnumeratePhysicalDevicesFailed)?
            .into_iter()
            .map(|device| -> Result<VulkanPhysicalDevice, VulkanError> {
                let properties = unsafe { instance.get_physical_device_properties(device) };

                let name = CStr::from_bytes_until_nul(unsafe {
                    std::slice::from_raw_parts(
                        properties.device_name.as_ptr().cast(),
                        properties.device_name.len(),
                    )
                })
                .map_err(|_| VulkanError::DeviceNameInvalid)?
                .to_str()
                .map_err(VulkanError::DeviceNameInvalidUtf8)?
                .to_owned();

                Ok(VulkanPhysicalDevice { device, name })
            })
            .collect::<Result<_, VulkanError>>()?;

        Ok(Self {
            entry,
            instance,
            devices,
        })
    }

    fn physical_devices(&self) -> &[Self::PhysicalDevice] {
        &self.devices
    }

    fn create_screen(&mut self) -> Result<Self::Screen, Self::Err> {
        todo!()
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

/// Implementation of [`Graphics::Err`] for Vulkan.
#[derive(Debug, Error)]
pub enum VulkanError {
    #[error("couldn't create Vulkan instance")]
    CreateInstanceFailed(#[source] ash::vk::Result),

    #[error("couldn't enumerate physical devices")]
    EnumeratePhysicalDevicesFailed(#[source] ash::vk::Result),

    #[error("no null byte in device name")]
    DeviceNameInvalid,

    #[error("device name is not valid UTF-8")]
    DeviceNameInvalidUtf8(#[source] std::str::Utf8Error),
}
