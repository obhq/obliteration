// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::engine::{builder, GraphicsError};

use crate::profile::Profile;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use winit::window::WindowAttributes;

#[cfg_attr(target_os = "macos", path = "metal/mod.rs")]
#[cfg_attr(not(target_os = "macos"), path = "vulkan/mod.rs")]
mod engine;

/// Provides method to build [`Graphics`].
pub trait EngineBuilder {
    type PhysicalDevice: PhysicalDevice;
    type Engine: Graphics;

    fn physical_devices(&self) -> &[Self::PhysicalDevice];

    /// Currently this method was designed to run only once per application lifetime.
    fn build(
        self,
        profile: &Profile,
        screen: WindowAttributes,
        shutdown: &Arc<AtomicBool>,
    ) -> Result<Arc<Self::Engine>, GraphicsError>;
}

pub trait PhysicalDevice: Sized {
    fn name(&self) -> &str;
}

/// The underlying graphics engine (e.g. Vulkan).
///
/// This trait act as a thin layer for graphics engine to be used by VMM. At compile-time this
/// layer will be optimized out and aggressively inlined the same as Hypervisor trait.
pub trait Graphics: Send + Sync + 'static {}
