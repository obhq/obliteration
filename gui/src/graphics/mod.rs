// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::engine::{new, GraphicsError};

use crate::profile::Profile;
use std::error::Error;
use std::sync::Arc;

#[cfg_attr(target_os = "macos", path = "metal/mod.rs")]
#[cfg_attr(not(target_os = "macos"), path = "vulkan/mod.rs")]
mod engine;

/// The underlying graphics engine (e.g. Vulkan).
pub trait Graphics: Sized + 'static {
    type PhysicalDevice: PhysicalDevice;
    type Screen: Screen;

    fn physical_devices(&self) -> &[Self::PhysicalDevice];
    fn create_screen(&mut self, profile: &Profile) -> Result<Self::Screen, GraphicsError>;
}

pub trait PhysicalDevice: Sized {
    fn name(&self) -> &str;
}

/// Encapsulates a platform-specific window for drawing a VM screen.
pub trait Screen: 'static {
    type Buffer: ScreenBuffer;
    type RunErr: Error;

    fn buffer(&self) -> &Arc<Self::Buffer>;
    fn run(self) -> Result<(), Self::RunErr>;
}

/// Manages off-screen buffers for [`Screen`].
///
/// How many buffering are available is depend on the implementation.
pub trait ScreenBuffer: Send + Sync {}
