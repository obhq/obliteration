pub const CPU_LEVEL_WHICH: i32 = 3;
pub const CPU_WHICH_TID: i32 = 1;

/// An implementation of `cpuset`.
#[derive(Debug)]
pub struct CpuSet {
    mask: CpuMask, // cs_mask
}

impl CpuSet {
    pub fn new(mask: CpuMask) -> Self {
        Self { mask }
    }

    pub fn mask(&self) -> &CpuMask {
        &self.mask
    }
}

/// An implementation of `cpuset_t`.
#[repr(C)]
#[derive(Debug, Default)]
pub struct CpuMask {
    pub bits: [u64; 1],
}
