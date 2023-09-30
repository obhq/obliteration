use crate::fs::VFileOps;

/// An implementation of `/dev/console`.
#[derive(Debug)]
pub struct Console {}

impl Console {
    pub fn new() -> Self {
        Self {}
    }
}

impl VFileOps for Console {}
