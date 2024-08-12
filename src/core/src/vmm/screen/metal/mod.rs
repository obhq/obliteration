use self::buffer::MetalBuffer;
use super::{Screen, ScreenBuffer, VmmError};
use crate::vmm::VmmScreen;
use metal::{CAMetalLayer, Device, MetalLayer};
use objc::runtime::{Object, NO, YES};
use objc::{msg_send, sel, sel_impl};
use std::ptr::null_mut;
use std::sync::Arc;
use thiserror::Error;

mod buffer;

/// Implementation of [`Screen`] using Metal.
///
/// Fields in this struct need to be dropped in a correct order.
pub struct Metal {
    view: *mut Object,
    buffer: Arc<MetalBuffer>,
    layer: MetalLayer,
    device: Device,
}

impl Metal {
    pub fn new(screen: *const VmmScreen) -> Result<Self, VmmError> {
        // Get Metal device.
        let device = match Device::system_default() {
            Some(v) => v,
            None => return Err(VmmError::GetMetalDeviceFailed),
        };

        // Setup Metal layer.
        let layer = MetalLayer::new();

        layer.set_device(&device);

        // Set view layer.
        let view = unsafe { (*screen).view as *mut Object };

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

impl Drop for Metal {
    fn drop(&mut self) {
        let l: *mut CAMetalLayer = null_mut();
        let _: () = unsafe { msg_send![self.view, setWantsLayer:NO] };
        let _: () = unsafe { msg_send![self.view, setLayer:l] };
    }
}

impl Screen for Metal {
    type Buffer = MetalBuffer;
    type UpdateErr = UpdateError;

    fn buffer(&self) -> &Arc<Self::Buffer> {
        &self.buffer
    }

    fn update(&mut self) -> Result<(), Self::UpdateErr> {
        todo!()
    }
}

/// Implementation of [`Screen::UpdateErr`].
#[derive(Debug, Error)]
pub enum UpdateError {}
