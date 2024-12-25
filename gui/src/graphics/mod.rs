// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::engine::{new, GraphicsError};

use crate::profile::Profile;
use crate::rt::Hook;
use std::rc::Rc;
use winit::window::WindowAttributes;

#[cfg_attr(target_os = "macos", path = "metal/mod.rs")]
#[cfg_attr(not(target_os = "macos"), path = "vulkan/mod.rs")]
mod engine;

/// The underlying graphics engine (e.g. Vulkan).
pub trait Graphics: Sized + 'static {
    type PhysicalDevice: PhysicalDevice;
    type Screen: Screen;

    fn physical_devices(&self) -> &[Self::PhysicalDevice];
    fn create_screen(
        &mut self,
        profile: &Profile,
        attrs: WindowAttributes,
    ) -> Result<Rc<Self::Screen>, GraphicsError>;
}

pub trait PhysicalDevice: Sized {
    fn name(&self) -> &str;
}

/// Encapsulates a platform-specific window for drawing a VM screen.
pub trait Screen: Hook {}
