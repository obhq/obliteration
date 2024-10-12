use super::VmmError;
use std::error::Error;
use std::sync::Arc;

#[cfg(target_os = "macos")]
mod metal;
#[cfg(not(target_os = "macos"))]
mod vulkan;

#[cfg(not(target_os = "macos"))]
pub type Default = self::vulkan::Vulkan;

#[cfg(target_os = "macos")]
pub type Default = self::metal::Metal;

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
