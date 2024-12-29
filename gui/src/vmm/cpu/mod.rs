// SPDX-License-Identifier: MIT OR Apache-2.0
use super::hw::DeviceContext;
use crate::hv::Cpu;
use std::collections::BTreeMap;
use std::num::NonZero;
use thiserror::Error;

pub mod debug;

/// Contains instantiated device context for a CPU.
pub struct Device<'a, C: Cpu> {
    pub context: Box<dyn DeviceContext<C> + 'a>,
    pub end: NonZero<usize>,
    pub name: &'a str,
}

impl<'a, C: Cpu> Device<'a, C> {
    pub fn insert<T: super::hw::Device>(
        tree: &mut BTreeMap<usize, Self>,
        dev: &'a T,
        f: impl FnOnce(&'a T) -> Box<dyn DeviceContext<C> + 'a>,
    ) {
        let addr = dev.addr();
        let dev = Self {
            context: f(dev),
            end: dev.len().checked_add(addr).unwrap(),
            name: dev.name(),
        };

        assert!(tree.insert(addr, dev).is_none());
    }
}

/// Implementation of [`gdbstub::target::Target::Error`].
#[derive(Debug, Error)]
pub enum GdbError {
    #[error("the main CPU exited")]
    MainCpuExited,

    #[error("CPU not found")]
    CpuNotFound,
}
