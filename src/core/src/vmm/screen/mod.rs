use std::error::Error;

#[cfg(target_os = "macos")]
mod metal;
#[cfg(not(target_os = "macos"))]
mod vulkan;

#[cfg(not(target_os = "macos"))]
pub type Default = self::vulkan::Vulkan;

#[cfg(target_os = "macos")]
pub type Default = self::metal::Metal;

/// Encapsulates a platform-specific surface for drawing a VM screen.
pub trait Screen: Send + Sync {
    type UpdateErr: Error;

    fn update(&self) -> Result<(), Self::UpdateErr>;
}
