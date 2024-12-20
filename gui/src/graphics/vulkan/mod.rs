// SPDX-License-Identifier: MIT OR Apache-2.0
use self::screen::VulkanScreen;
use super::Graphics;
use crate::profile::Profile;
use ash::vk::{ApplicationInfo, InstanceCreateInfo, QueueFlags, API_VERSION_1_3};
use std::ffi::CStr;
use thiserror::Error;

mod buffer;
mod screen;

pub fn new() -> Result<impl Graphics, GraphicsError> {
    // Setup application info.
    let mut app = ApplicationInfo::default();

    app.p_application_name = c"Obliteration".as_ptr();
    app.api_version = API_VERSION_1_3;

    // Setup validation layers.
    let layers = [
        #[cfg(debug_assertions)]
        c"VK_LAYER_KHRONOS_validation".as_ptr(),
    ];

    // Setup VkInstanceCreateInfo.
    let mut info = InstanceCreateInfo::default();

    info.p_application_info = &app;
    info.pp_enabled_layer_names = layers.as_ptr();
    info.enabled_layer_count = layers.len().try_into().unwrap();

    // Create Vulkan instance.
    let api = ash::Entry::linked();
    let mut vk = match unsafe { api.create_instance(&info, None) } {
        Ok(instance) => Vulkan {
            instance,
            devices: Vec::new(),
        },
        Err(e) => return Err(GraphicsError::CreateInstance(e)),
    };

    // List available devices.
    let all = match unsafe { vk.instance.enumerate_physical_devices() } {
        Ok(v) => v,
        Err(e) => return Err(GraphicsError::EnumeratePhysicalDevices(e)),
    };

    if all.is_empty() {
        return Err(GraphicsError::NoPhysicalDevice);
    }

    for dev in all {
        // Filter out devices without Vulkan 1.3.
        let p = unsafe { vk.instance.get_physical_device_properties(dev) };

        if p.api_version < API_VERSION_1_3 {
            continue;
        }

        // Skip if device does not support graphics operations.
        if !unsafe { vk.instance.get_physical_device_queue_family_properties(dev) }
            .iter()
            .any(|p| p.queue_flags.contains(QueueFlags::GRAPHICS))
        {
            continue;
        }

        // Add to list.
        let name = unsafe { CStr::from_ptr(p.device_name.as_ptr()) }
            .to_str()
            .unwrap()
            .to_owned();

        vk.devices.push(PhysicalDevice { device: dev, name });
    }

    if vk.devices.is_empty() {
        return Err(GraphicsError::NoSuitableDevice);
    }

    Ok(vk)
}

/// Implementation of [`Graphics`] using Vulkan.
struct Vulkan {
    instance: ash::Instance,
    devices: Vec<PhysicalDevice>,
}

impl Graphics for Vulkan {
    type PhysicalDevice = PhysicalDevice;
    type Screen = VulkanScreen;

    fn physical_devices(&self) -> &[Self::PhysicalDevice] {
        &self.devices
    }

    fn create_screen(&mut self, profile: &Profile) -> Result<Self::Screen, GraphicsError> {
        todo!()
    }
}

impl Drop for Vulkan {
    fn drop(&mut self) {
        unsafe { self.instance.destroy_instance(None) };
    }
}

pub struct PhysicalDevice {
    device: ash::vk::PhysicalDevice,
    name: String,
}

impl super::PhysicalDevice for PhysicalDevice {
    fn name(&self) -> &str {
        &self.name
    }
}

/// Represents an error when operation on Vulkan fails.
#[derive(Debug, Error)]
pub enum GraphicsError {
    #[error("couldn't create Vulkan instance")]
    CreateInstance(#[source] ash::vk::Result),

    #[error("couldn't enumerate physical devices")]
    EnumeratePhysicalDevices(#[source] ash::vk::Result),

    #[error("no any Vulkan physical device available")]
    NoPhysicalDevice,

    #[error("no Vulkan device supports graphics operations with Vulkan 1.3")]
    NoSuitableDevice,
}
