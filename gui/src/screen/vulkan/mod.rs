// SPDX-License-Identifier: MIT OR Apache-2.0
use self::buffer::VulkanBuffer;
use super::{Screen, ScreenBuffer};
use crate::vmm::VmmScreen;
use ash::vk::{
    ApplicationInfo, DeviceCreateInfo, DeviceQueueCreateInfo, Handle, InstanceCreateInfo,
    QueueFlags,
};
use ash::Device;
use std::ffi::CStr;
use std::sync::Arc;
use thiserror::Error;

mod buffer;

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

/// Implementation of [`Screen`] using Vulkan.
pub struct VulkanScreen {
    buffer: Arc<VulkanBuffer>,
    device: Device,
}

impl VulkanScreen {
    pub fn from_screen(screen: &VmmScreen) -> Result<Self, VulkanScreenError> {
        let entry = ash::Entry::linked();

        let instance = unsafe {
            ash::Instance::load(
                entry.static_fn(),
                ash::vk::Instance::from_raw(screen.vk_instance.try_into().unwrap()),
            )
        };

        // Wrap VkPhysicalDevice.
        let physical = screen.vk_device.try_into().unwrap();
        let physical = ash::vk::PhysicalDevice::from_raw(physical);

        // Setup VkDeviceQueueCreateInfo.
        let queue = unsafe { instance.get_physical_device_queue_family_properties(physical) }
            .into_iter()
            .position(|p| p.queue_flags.contains(QueueFlags::GRAPHICS))
            .ok_or(VulkanScreenError::NoQueue)?;

        let queue = queue
            .try_into()
            .map_err(|_| VulkanScreenError::QueueOutOfBounds(queue))?;

        let queues = DeviceQueueCreateInfo::default()
            .queue_family_index(queue)
            .queue_priorities(&[1.0]);

        // Create logical device.
        let device = DeviceCreateInfo::default().queue_create_infos(std::slice::from_ref(&queues));
        let device = unsafe { instance.create_device(physical, &device, None) }
            .map_err(VulkanScreenError::CreateDeviceFailed)?;

        Ok(Self {
            buffer: Arc::new(VulkanBuffer::new()),
            device,
        })
    }
}

impl Drop for VulkanScreen {
    fn drop(&mut self) {
        unsafe { self.device.device_wait_idle().unwrap() };
        unsafe { self.device.destroy_device(None) };
    }
}

impl Screen for VulkanScreen {
    type Buffer = VulkanBuffer;
    type UpdateErr = UpdateError;

    fn buffer(&self) -> &Arc<Self::Buffer> {
        &self.buffer
    }

    fn update(&mut self) -> Result<(), Self::UpdateErr> {
        Ok(())
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

/// Represents an error when [`VulkanScreen::new()`] fails.
#[derive(Debug, Error)]
pub enum VulkanScreenError {
    #[error("couldn't find suitable queue")]
    NoQueue,

    #[error("queue index #{0} out of bounds")]
    QueueOutOfBounds(usize),

    #[error("couldn't create a logical device")]
    CreateDeviceFailed(#[source] ash::vk::Result),
}

/// Implementation of [`Screen::UpdateErr`].
#[derive(Debug, Error)]
pub enum UpdateError {}
