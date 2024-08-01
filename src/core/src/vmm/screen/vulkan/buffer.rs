use super::ScreenBuffer;

/// Manages Vulkan off-screen buffers.
pub struct VulkanBuffer {}

impl VulkanBuffer {
    pub fn new() -> Self {
        Self {}
    }
}

impl ScreenBuffer for VulkanBuffer {}
