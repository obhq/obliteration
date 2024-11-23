// SPDX-License-Identifier: MIT OR Apache-2.0
use super::buffer::MetalBuffer;
use crate::graphics::Screen;
use crate::vmm::VmmScreen;
use metal::{CAMetalLayer, Device, MetalLayer};
use objc::runtime::{Object, NO, YES};
use objc::{msg_send, sel, sel_impl};
use std::ptr::null_mut;
use std::sync::Arc;
use thiserror::Error;

/// Implementation of [`Screen`] using Metal.
///
/// Fields in this struct need to be dropped in a correct order.
pub struct MetalScreen {
    view: *mut Object,
    buffer: Arc<MetalBuffer>,
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
            buffer: Arc::new(MetalBuffer::new()),
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

impl Screen for MetalScreen {
    type Buffer = MetalBuffer;
    type RunErr = RunError;

    fn buffer(&self) -> &Arc<Self::Buffer> {
        &self.buffer
    }

    fn run(&mut self) -> Result<(), Self::RunErr> {
        todo!()
    }
}

/// Represents an error when [`MetalScreen::new()`] fails.
#[derive(Debug, Error)]
pub enum MetalError {
    #[error("couldn't get default MTLDevice")]
    GetDeviceFailed,
}

/// Implementation of [`Screen::RunErr`].
#[derive(Debug, Error)]
pub enum RunError {}
