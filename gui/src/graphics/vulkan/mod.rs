// SPDX-License-Identifier: MIT OR Apache-2.0
use self::screen::VulkanScreen;
use self::window::VulkanWindow;
use super::Graphics;
use crate::profile::Profile;
use crate::rt::{create_window, raw_display_handle, RuntimeError};
use ash::extensions::khr::Surface;
use ash::vk::{ApplicationInfo, InstanceCreateInfo, QueueFlags, API_VERSION_1_3};
use ash::{Entry, Instance};
use ash_window::enumerate_required_extensions;
use std::ffi::CStr;
use std::mem::ManuallyDrop;
use std::sync::Arc;
use thiserror::Error;
use winit::window::WindowAttributes;

mod screen;
mod window;

pub fn new() -> Result<impl Graphics, GraphicsError> {
    // Get required extensions for window.
    let exts = enumerate_required_extensions(raw_display_handle())
        .map_err(GraphicsError::GetExtensionsForWindow)?;

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
    let info = InstanceCreateInfo::builder()
        .application_info(&app)
        .enabled_layer_names(&layers)
        .enabled_extension_names(exts);

    // Create Vulkan instance.
    let entry = Entry::linked();
    let mut vk = match unsafe { entry.create_instance(&info, None) } {
        Ok(instance) => Vulkan {
            devices: ManuallyDrop::new(Vec::new()),
            surface: ManuallyDrop::new(Surface::new(&entry, &instance)),
            instance,
            entry,
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
///
/// Fields in this struct need to drop in a correct order.
struct Vulkan {
    devices: ManuallyDrop<Vec<PhysicalDevice>>,
    surface: ManuallyDrop<Surface>,
    instance: Instance,
    entry: Entry,
}

impl Graphics for Vulkan {
    type PhysicalDevice = PhysicalDevice;
    type Screen = VulkanScreen;

    fn physical_devices(&self) -> &[Self::PhysicalDevice] {
        &self.devices
    }

    fn create_screen(
        self,
        profile: &Profile,
        attrs: WindowAttributes,
    ) -> Result<Arc<Self::Screen>, GraphicsError> {
        let screen = VulkanScreen::new(self, profile).map(Arc::new)?;
        let window = create_window(attrs, |w| VulkanWindow::new(&screen, w))
            .map_err(GraphicsError::CreateWindow)?;

        crate::rt::push_hook(window);

        Ok(screen)
    }
}

impl Drop for Vulkan {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.devices) };
        unsafe { ManuallyDrop::drop(&mut self.surface) };
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
    #[error("couldn't get required Vulkan extensions for window")]
    GetExtensionsForWindow(#[source] ash::vk::Result),

    #[error("couldn't create Vulkan instance")]
    CreateInstance(#[source] ash::vk::Result),

    #[error("couldn't enumerate physical devices")]
    EnumeratePhysicalDevices(#[source] ash::vk::Result),

    #[error("no any Vulkan physical device available")]
    NoPhysicalDevice,

    #[error("no Vulkan device supports graphics operations with Vulkan 1.3")]
    NoSuitableDevice,

    #[error("couldn't create a logical device")]
    CreateDevice(#[source] ash::vk::Result),

    #[error("couldn't create Vulkan surface")]
    CreateSurface(#[source] ash::vk::Result),

    #[error("couldn't create window")]
    CreateWindow(#[source] RuntimeError),
}
