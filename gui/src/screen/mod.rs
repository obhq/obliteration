// SPDX-License-Identifier: MIT OR Apache-2.0
use std::error::Error;
use std::sync::Arc;

#[cfg_attr(target_os = "macos", path = "metal/mod.rs")]
#[cfg_attr(not(target_os = "macos"), path = "vulkan/mod.rs")]
mod engine;

#[cfg(not(target_os = "macos"))]
pub type Default = self::engine::Vulkan;

#[cfg(target_os = "macos")]
pub type Default = self::engine::Metal;

#[cfg(not(target_os = "macos"))]
pub type ScreenError = self::engine::VulkanError;

#[cfg(target_os = "macos")]
pub type ScreenError = self::engine::MetalError;

/// Encapsulates a platform-specific surface for drawing a VM screen.
pub trait Screen: 'static {
    type Buffer: ScreenBuffer;
    type UpdateErr: Error;

    fn buffer(&self) -> &Arc<Self::Buffer>;
    fn update(&mut self) -> Result<(), Self::UpdateErr>;
}

/// Manages off-screen buffers for [`Screen`].
///
/// How many buffering are available is depend on the implementation.
pub trait ScreenBuffer: Send + Sync {}
