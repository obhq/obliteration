use std::num::NonZeroI32;

/// An implementation of `pgrp` struct.
#[derive(Debug)]
pub struct VProcGroup {
    id: NonZeroI32,
}

impl VProcGroup {
    pub fn new(id: NonZeroI32) -> Self {
        Self { id }
    }
}
