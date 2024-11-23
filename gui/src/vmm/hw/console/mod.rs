// SPDX-License-Identifier: MIT OR Apache-2.0
use self::context::Context;
use super::{Device, DeviceContext};
use crate::hv::Hypervisor;
use crate::vmm::VmmHandler;
use obconf::ConsoleMemory;
use std::num::NonZero;

mod context;

/// Virtual console for the VM.
pub struct Console {
    addr: usize,
    len: NonZero<usize>,
}

impl Console {
    pub fn new(addr: usize, block_size: NonZero<usize>) -> Self {
        let len = size_of::<ConsoleMemory>()
            .checked_next_multiple_of(block_size.get())
            .and_then(NonZero::new)
            .unwrap();

        Self { addr, len }
    }

    pub fn create_context<'a, H: Hypervisor, E: VmmHandler>(
        &'a self,
        hv: &'a H,
        handler: &'a E,
    ) -> Box<dyn DeviceContext<H::Cpu<'a>> + 'a> {
        Box::new(Context::new(self, hv, handler))
    }
}

impl Device for Console {
    fn name(&self) -> &str {
        "Virtual Console"
    }

    fn addr(&self) -> usize {
        self.addr
    }

    fn len(&self) -> NonZero<usize> {
        self.len
    }
}
