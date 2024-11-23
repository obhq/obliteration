// SPDX-License-Identifier: MIT OR Apache-2.0
use std::error::Error;
use std::sync::Arc;

#[cfg_attr(target_os = "macos", path = "metal/mod.rs")]
#[cfg_attr(not(target_os = "macos"), path = "vulkan/mod.rs")]
mod engine;

#[cfg(not(target_os = "macos"))]
pub type DefaultApi = self::engine::Vulkan;

#[cfg(target_os = "macos")]
pub type DefaultApi = self::engine::Metal;

/// The underlying graphics engine (e.g. Vulkan).
pub trait GraphicsApi: Sized + 'static {
    type PhysicalDevice: PhysicalDevice;

    type CreateError: core::error::Error;

    fn new() -> Result<Self, Self::CreateError>;

    fn physical_devices(&self) -> &[Self::PhysicalDevice];
}

pub trait PhysicalDevice: Sized {
    fn name(&self) -> &str;
}

/// Encapsulates a platform-specific window for drawing a VM screen.
pub trait Screen: 'static {
    type Buffer: ScreenBuffer;
    type RunErr: Error;

    fn buffer(&self) -> &Arc<Self::Buffer>;
    fn run(&mut self) -> Result<(), Self::RunErr>;
}

/// Manages off-screen buffers for [`Screen`].
///
/// How many buffering are available is depend on the implementation.
pub trait ScreenBuffer: Send + Sync {}
