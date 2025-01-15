// SPDX-License-Identifier: MIT OR Apache-2.0
use super::{GraphicsError, VulkanBuilder};
use crate::graphics::Graphics;
use crate::profile::Profile;
use ash::extensions::khr::{WaylandSurface, Win32Surface, XcbSurface, XlibSurface};
use ash::vk::{
    DeviceCreateInfo, DeviceQueueCreateInfo, QueueFlags, SurfaceKHR, WaylandSurfaceCreateInfoKHR,
    Win32SurfaceCreateInfoKHR, XcbSurfaceCreateInfoKHR, XlibSurfaceCreateInfoKHR,
};
use ash::Device;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use winit::window::Window;

/// Implementation of [`Graphics`] using Vulkan.
///
/// Fields in this struct must be dropped in a correct order.
pub struct Vulkan {
    device: Device,
    builder: VulkanBuilder,
}

impl Vulkan {
    pub fn new(b: VulkanBuilder, profile: &Profile) -> Result<Self, GraphicsError> {
        // TODO: Use selected device.
        let physical = b.devices.first().unwrap().device;

        // Setup VkDeviceQueueCreateInfo.
        let instance = &b.instance;
        let queue = unsafe { instance.get_physical_device_queue_family_properties(physical) }
            .into_iter()
            .position(|p| p.queue_flags.contains(QueueFlags::GRAPHICS))
            .unwrap(); // We required all selectable devices to support graphics operations.

        let mut queues = DeviceQueueCreateInfo::default();
        let priorities = [1.0];

        queues.queue_family_index = queue.try_into().unwrap();
        queues.queue_count = 1;
        queues.p_queue_priorities = priorities.as_ptr();

        // Setup VkDeviceCreateInfo.
        let mut device = DeviceCreateInfo::default();

        device.p_queue_create_infos = &queues;
        device.queue_create_info_count = 1;

        // Create logical device.
        let device = unsafe { instance.create_device(physical, &device, None) }
            .map_err(GraphicsError::CreateDevice)?;

        Ok(Self { device, builder: b })
    }

    /// # Safety
    /// The returned [`SurfaceKHR`] must be destroyed before `win` and this [`Vulkan`].
    pub unsafe fn create_surface(&self, win: &Window) -> Result<SurfaceKHR, ash::vk::Result> {
        let e = &self.builder.entry;
        let i = &self.builder.instance;
        let w = win.window_handle().unwrap();

        match w.as_ref() {
            RawWindowHandle::UiKit(_)
            | RawWindowHandle::AppKit(_)
            | RawWindowHandle::Web(_)
            | RawWindowHandle::WebCanvas(_)
            | RawWindowHandle::WebOffscreenCanvas(_) => {
                unreachable!()
            }
            RawWindowHandle::Xlib(v) => {
                let c = XlibSurfaceCreateInfoKHR::builder()
                    .dpy(match win.display_handle().unwrap().as_ref() {
                        RawDisplayHandle::Xlib(v) => v.display.unwrap().as_ptr().cast(),
                        _ => unreachable!(),
                    })
                    .window(v.window);

                XlibSurface::new(e, i).create_xlib_surface(&c, None)
            }
            RawWindowHandle::Xcb(v) => {
                let c = XcbSurfaceCreateInfoKHR::builder()
                    .connection(match win.display_handle().unwrap().as_ref() {
                        RawDisplayHandle::Xcb(v) => v.connection.unwrap().as_ptr(),
                        _ => unreachable!(),
                    })
                    .window(v.window.get());

                XcbSurface::new(e, i).create_xcb_surface(&c, None)
            }
            RawWindowHandle::Wayland(v) => {
                let c = WaylandSurfaceCreateInfoKHR::builder()
                    .display(match win.display_handle().unwrap().as_ref() {
                        RawDisplayHandle::Wayland(v) => v.display.as_ptr(),
                        _ => unreachable!(),
                    })
                    .surface(v.surface.as_ptr());

                WaylandSurface::new(e, i).create_wayland_surface(&c, None)
            }
            RawWindowHandle::Win32(v) => {
                let c = Win32SurfaceCreateInfoKHR::builder()
                    .hinstance(v.hinstance.unwrap().get() as _)
                    .hwnd(v.hwnd.get() as _);

                Win32Surface::new(e, i).create_win32_surface(&c, None)
            }
            _ => todo!(),
        }
    }

    /// # Safety
    /// See `vkDestroySurfaceKHR` docs for valid usage.
    pub unsafe fn destroy_surface(&self, surface: SurfaceKHR) {
        self.builder.surface.destroy_surface(surface, None);
    }
}

impl Drop for Vulkan {
    fn drop(&mut self) {
        // Free device.
        unsafe { self.device.device_wait_idle().unwrap() };
        unsafe { self.device.destroy_device(None) };
    }
}

impl Graphics for Vulkan {}
