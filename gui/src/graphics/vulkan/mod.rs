// SPDX-License-Identifier: MIT OR Apache-2.0
use self::engine::Vulkan;
use self::window::VulkanWindow;
use super::GraphicsBuilder;
use crate::profile::Profile;
use crate::settings::Settings;
use ash::extensions::khr::Surface;
use ash::vk::{
    API_VERSION_1_3, ApplicationInfo, InstanceCreateInfo, PhysicalDeviceIDProperties,
    PhysicalDeviceProperties2, QueueFlags,
};
use ash::{Entry, Instance};
use raw_window_handle::RawDisplayHandle;
use std::ffi::CStr;
use std::mem::ManuallyDrop;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use thiserror::Error;
use winit::window::WindowAttributes;

// Clippy lints this because the module is named the same as the parent module
#[allow(clippy::module_inception)]
mod engine;
mod window;

pub fn builder(settings: &Settings) -> Result<impl GraphicsBuilder, GraphicsError> {
    // Get required extensions for window.
    let window_ext = match wae::raw_display_handle() {
        RawDisplayHandle::UiKit(_) | RawDisplayHandle::AppKit(_) | RawDisplayHandle::Web(_) => {
            unreachable!()
        }
        RawDisplayHandle::Xlib(_) => c"VK_KHR_xlib_surface",
        RawDisplayHandle::Xcb(_) => c"VK_KHR_xcb_surface",
        RawDisplayHandle::Wayland(_) => c"VK_KHR_wayland_surface",
        RawDisplayHandle::Windows(_) => c"VK_KHR_win32_surface",
        _ => todo!(),
    };

    let exts = [c"VK_KHR_surface".as_ptr(), window_ext.as_ptr()];

    // Setup application info.
    let app = ApplicationInfo {
        p_application_name: c"Obliteration".as_ptr(),
        api_version: API_VERSION_1_3,
        ..Default::default()
    };

    // Setup validation layers.
    let layers: &[*const core::ffi::c_char] = if settings.graphics_debug_layer() {
        &[c"VK_LAYER_KHRONOS_validation".as_ptr()]
    } else {
        &[]
    };

    // Setup VkInstanceCreateInfo.
    let info = InstanceCreateInfo::builder()
        .application_info(&app)
        .enabled_layer_names(layers)
        .enabled_extension_names(&exts);

    // Create Vulkan instance.
    let entry = Entry::linked();
    let mut b = match unsafe { entry.create_instance(&info, None) } {
        Ok(instance) => VulkanBuilder {
            devices: ManuallyDrop::new(Vec::new()),
            surface: ManuallyDrop::new(Surface::new(&entry, &instance)),
            instance,
            entry,
        },
        Err(e) => return Err(GraphicsError::CreateInstance(e)),
    };

    // List available devices.
    let all = match unsafe { b.instance.enumerate_physical_devices() } {
        Ok(v) => v,
        Err(e) => return Err(GraphicsError::EnumeratePhysicalDevices(e)),
    };

    if all.is_empty() {
        return Err(GraphicsError::NoPhysicalDevice);
    }

    for dev in all {
        // Filter out devices without Vulkan 1.3.
        let mut id = PhysicalDeviceIDProperties::builder();
        let mut p2 = PhysicalDeviceProperties2::builder().push_next(&mut id);

        unsafe { b.instance.get_physical_device_properties2(dev, &mut p2) };

        if p2.properties.api_version < API_VERSION_1_3 {
            continue;
        }

        // Skip if device does not support graphics operations.
        if !unsafe { b.instance.get_physical_device_queue_family_properties(dev) }
            .iter()
            .any(|p| p.queue_flags.contains(QueueFlags::GRAPHICS))
        {
            continue;
        }

        // Add to list.
        let name = unsafe { CStr::from_ptr(p2.properties.device_name.as_ptr()) }
            .to_str()
            .unwrap()
            .to_owned();

        b.devices.push(PhysicalDevice {
            device: dev,
            id: id.device_uuid,
            name,
        });
    }

    if b.devices.is_empty() {
        return Err(GraphicsError::NoSuitableDevice);
    }

    Ok(b)
}

/// Implementation of [`GraphicsBuilder`] for Vulkan.
///
/// Fields in this struct need to drop in a correct order.
struct VulkanBuilder {
    devices: ManuallyDrop<Vec<PhysicalDevice>>,
    surface: ManuallyDrop<Surface>,
    instance: Instance,
    entry: Entry,
}

impl GraphicsBuilder for VulkanBuilder {
    type PhysicalDevice = PhysicalDevice;
    type Engine = Vulkan;

    fn physical_devices(&self) -> &[Self::PhysicalDevice] {
        &self.devices
    }

    fn build(
        self,
        profile: &Profile,
        screen: WindowAttributes,
        shutdown: &Arc<AtomicBool>,
    ) -> Result<Arc<Self::Engine>, GraphicsError> {
        let engine = Vulkan::new(self, profile).map(Arc::new)?;
        let window = wae::create_window(screen).map_err(GraphicsError::CreateWindow)?;
        let window = VulkanWindow::new(&engine, window, shutdown).map(Rc::new)?;

        wae::register_window(&window);
        wae::push_hook(window);

        Ok(engine)
    }
}

impl Drop for VulkanBuilder {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.devices) };
        unsafe { ManuallyDrop::drop(&mut self.surface) };
        unsafe { self.instance.destroy_instance(None) };
    }
}

/// Implementation of [`super::PhysicalDevice`].
pub struct PhysicalDevice {
    device: ash::vk::PhysicalDevice,
    id: [u8; 16],
    name: String,
}

impl super::PhysicalDevice for PhysicalDevice {
    fn id(&self) -> &[u8] {
        &self.id
    }

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

    #[error("no Vulkan physical devices available")]
    NoPhysicalDevice,

    #[error("no Vulkan device supports graphics operations with Vulkan 1.3")]
    NoSuitableDevice,

    #[error("couldn't create a logical device")]
    CreateDevice(#[source] ash::vk::Result),

    #[error("couldn't create Vulkan surface")]
    CreateSurface(#[source] ash::vk::Result),

    #[error("couldn't create window")]
    CreateWindow(#[source] winit::error::OsError),
}
