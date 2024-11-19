// SPDX-License-Identifier: MIT OR Apache-2.0
use self::context::Context;
use super::{Device, DeviceContext};
use crate::hv::Cpu;
use crate::vmm::VmmEventHandler;
use obconf::VmmMemory;
use std::num::NonZero;

mod context;

/// Virtual device for the kernel to communicate with the VMM.
pub struct Vmm {
    addr: usize,
    len: NonZero<usize>,
    event: VmmEventHandler,
}

impl Vmm {
    pub fn new(addr: usize, block_size: NonZero<usize>, event: VmmEventHandler) -> Self {
        let len = size_of::<VmmMemory>()
            .checked_next_multiple_of(block_size.get())
            .and_then(NonZero::new)
            .unwrap();

        Self { addr, len, event }
    }

    pub fn create_context<C: Cpu>(&self) -> Box<dyn DeviceContext<C> + '_> {
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
