// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::graphics::Screen;
use crate::rt::{Hook, RuntimeWindow};
use crate::vmm::VmmScreen;
use metal::{CAMetalLayer, Device, MetalLayer};
use objc::runtime::{Object, NO, YES};
use objc::{msg_send, sel, sel_impl};
use std::error::Error;
use std::ptr::null_mut;
use std::sync::Arc;
use thiserror::Error;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceId, ElementState, InnerSizeWriter, MouseButton, StartCause};
use winit::event_loop::ControlFlow;
use winit::window::WindowId;

/// Implementation of [`Screen`] using Metal.
///
/// Fields in this struct need to be dropped in a correct order.
pub struct MetalScreen {
    view: *mut Object,
    layer: MetalLayer,
    device: Device,
}

impl MetalScreen {
    pub fn new() -> Result<Self, MetalError> {
        todo!()
    }

    pub fn from_screen(screen: &VmmScreen) -> Result<Self, MetalError> {
        // Get Metal device.
        let device = match Device::system_default() {
            Some(v) => v,
            None => return Err(MetalError::GetDeviceFailed),
        };

        // Setup Metal layer.
        let layer = MetalLayer::new();

        layer.set_device(&device);

        // Set view layer.
        let view = screen.view as *mut Object;

        let _: () = unsafe { msg_send![view, setLayer:layer.as_ref()] };
        let _: () = unsafe { msg_send![view, setWantsLayer:YES] };

        Ok(Self {
            view,
            layer,
            device,
        })
    }
}

impl Drop for MetalScreen {
    fn drop(&mut self) {
        let l: *mut CAMetalLayer = null_mut();
        let _: () = unsafe { msg_send![self.view, setWantsLayer:NO] };
        let _: () = unsafe { msg_send![self.view, setLayer:l] };
    }
}

impl RuntimeWindow for MetalScreen {
    fn on_resized(&self, new: PhysicalSize<u32>) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_close_requested(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_focused(&self, gained: bool) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_cursor_moved(
        &self,
        dev: DeviceId,
        pos: PhysicalPosition<f64>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_cursor_left(&self, dev: DeviceId) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_mouse_input(
        &self,
        dev: DeviceId,
        st: ElementState,
        btn: MouseButton,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_scale_factor_changed(
        &self,
        new: f64,
        sw: InnerSizeWriter,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_redraw_requested(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }
}

impl Hook for MetalScreen {
    fn new_events(&self, cause: &StartCause) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn pre_window_event(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn window_destroyed(&self, id: WindowId) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn post_window_event(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn about_to_wait(&self) -> Result<ControlFlow, Box<dyn Error + Send + Sync>> {
        todo!()
    }
}

impl Screen for MetalScreen {}

/// Represents an error when [`MetalScreen::new()`] fails.
#[derive(Debug, Error)]
pub enum MetalError {
    #[error("couldn't get default MTLDevice")]
    GetDeviceFailed,
}
