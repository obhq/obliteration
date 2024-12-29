// SPDX-License-Identifier: MIT OR Apache-2.0
use self::context::Context;
use super::{Device, DeviceContext};
use crate::hv::Cpu;
use obconf::VmmMemory;
use std::num::NonZero;

mod context;

/// Virtual device for the kernel to communicate with the VMM.
pub struct Vmm {
    addr: usize,
    len: NonZero<usize>,
}

impl Vmm {
    pub fn new(addr: usize, block_size: NonZero<usize>) -> Self {
        let len = size_of::<VmmMemory>()
            .checked_next_multiple_of(block_size.get())
            .and_then(NonZero::new)
            .unwrap();

        Self { addr, len }
    }

    pub fn create_context<'a, C: Cpu>(&'a self) -> Box<dyn DeviceContext<C> + 'a> {
        Box::new(Context::new(self))
    }
}

impl Device for Vmm {
    fn name(&self) -> &str {
        "VMM"
    }

    fn addr(&self) -> usize {
        self.addr
    }

    fn len(&self) -> NonZero<usize> {
        self.len
    }
}
