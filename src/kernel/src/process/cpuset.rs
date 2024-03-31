use crate::{syscalls::SysErr, Errno};

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

/// An implementation of `cpulevel_t`.
#[derive(Debug, Clone, Copy)]
#[repr(i32)]
pub(super) enum CpuLevel {
    Root = 1,
    Cpuset = 2,
    Which = 3,
}

impl TryFrom<i32> for CpuLevel {
    type Error = SysErr;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Root),
            2 => Ok(Self::Cpuset),
            3 => Ok(Self::Which),
            _ => Err(SysErr::Raw(Errno::EINVAL)),
        }
    }
}

/// An implementation of `cpuwhich_t`.
#[derive(Debug, Clone, Copy)]
#[repr(i32)]
pub(super) enum CpuWhich {
    Tid = 1,
    Pid = 2,
    Cpuset = 3,
    Irq = 4,
    Jail = 5,
}

impl TryFrom<i32> for CpuWhich {
    type Error = SysErr;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Tid),
            2 => Ok(Self::Pid),
            3 => Ok(Self::Cpuset),
            4 => Ok(Self::Irq),
            5 => Ok(Self::Jail),
            _ => Err(SysErr::Raw(Errno::EINVAL)),
        }
    }
}
