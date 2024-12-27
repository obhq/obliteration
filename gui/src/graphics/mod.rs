// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::engine::{new, GraphicsError};

use crate::profile::Profile;
use std::sync::Arc;
use winit::window::WindowAttributes;

#[cfg_attr(target_os = "macos", path = "metal/mod.rs")]
#[cfg_attr(not(target_os = "macos"), path = "vulkan/mod.rs")]
mod engine;

/// The underlying graphics engine (e.g. Vulkan).
pub trait Graphics: Sized + 'static {
    type PhysicalDevice: PhysicalDevice;
    type Screen: Screen;

    fn physical_devices(&self) -> &[Self::PhysicalDevice];

    /// Currently this method was designed to run only once per application lifetime.
    fn create_screen(
        self,
        profile: &Profile,
        attrs: WindowAttributes,
    ) -> Result<Arc<Self::Screen>, GraphicsError>;
}

pub trait PhysicalDevice: Sized {
    fn name(&self) -> &str;
}

/// Encapsulates a platform-specific window for drawing a VM screen.
///
/// This trait act as a thin layer for graphics engine for the VMM to use. At compile-time this
/// layer will be optimized out and aggressively inlined the same as Hypervisor trait.
pub trait Screen: Send + Sync + 'static {}
