// SPDX-License-Identifier: MIT OR Apache-2.0
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
