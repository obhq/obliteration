use crate::errno::EINVAL;
use crate::syscalls::{SysArg, SysErr};

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
#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub(super) enum CpuLevel {
    Root = 1,
    Cpuset = 2,
    Which = 3,
}

impl CpuLevel {
    pub fn new(v: i32) -> Option<Self> {
        Some(match v {
            1 => Self::Root,
            2 => Self::Cpuset,
            3 => Self::Which,
            _ => return None,
        })
    }
}

impl TryFrom<SysArg> for CpuLevel {
    type Error = SysErr;

    fn try_from(value: SysArg) -> Result<Self, Self::Error> {
        value
            .try_into()
            .ok()
            .and_then(|v| Self::new(v))
            .ok_or(SysErr::Raw(EINVAL))
    }
}

/// An implementation of `cpuwhich_t`.
#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub(super) enum CpuWhich {
    Tid = 1,
    Pid = 2,
    Cpuset = 3,
    Irq = 4,
    Jail = 5,
}

impl CpuWhich {
    pub fn new(v: i32) -> Option<Self> {
        Some(match v {
            1 => Self::Tid,
            2 => Self::Pid,
            3 => Self::Cpuset,
            4 => Self::Irq,
            5 => Self::Jail,
            _ => return None,
        })
    }
}

impl TryFrom<SysArg> for CpuWhich {
    type Error = SysErr;

    fn try_from(value: SysArg) -> Result<Self, Self::Error> {
        value
            .try_into()
            .ok()
            .and_then(|v| Self::new(v))
            .ok_or(SysErr::Raw(EINVAL))
    }
}
