// SPDX-License-Identifier: MIT OR Apache-2.0
use std::error::Error;
use std::sync::Arc;

#[cfg_attr(target_os = "macos", path = "metal/mod.rs")]
#[cfg_attr(not(target_os = "macos"), path = "vulkan/mod.rs")]
mod engine;

#[cfg(not(target_os = "macos"))]
pub type DefaultScreen = self::engine::VulkanScreen;

#[cfg(target_os = "macos")]
pub type DefaultScreen = self::engine::MetalScreen;

#[cfg(not(target_os = "macos"))]
pub type ScreenError = self::engine::VulkanScreenError;

#[cfg(target_os = "macos")]
pub type ScreenError = self::engine::MetalError;

/// Encapsulates a platform-specific surface for drawing a VM screen.
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
