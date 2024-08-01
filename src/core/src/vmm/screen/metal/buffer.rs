use super::ScreenBuffer;

/// Manages Metal off-screen buffers.
pub struct MetalBuffer {}

impl MetalBuffer {
    pub fn new() -> Self {
        Self {}
    }
}

impl ScreenBuffer for MetalBuffer {}
