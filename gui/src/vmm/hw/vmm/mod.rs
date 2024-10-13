// SPDX-License-Identifier: MIT OR Apache-2.0
use self::context::Context;
use super::{Device, DeviceContext};
use crate::vmm::cpu::CpuState;
use crate::vmm::hv::Hypervisor;
use crate::vmm::VmmEventHandler;
use obconf::VmmMemory;
use std::num::NonZero;
use std::sync::Mutex;

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
}

impl<H: Hypervisor> Device<H> for Vmm {
    fn addr(&self) -> usize {
        self.addr
    }

    fn len(&self) -> NonZero<usize> {
        self.len
    }

    fn create_context<'a>(
        &'a self,
        _: &'a H,
        _: &'a Mutex<CpuState>,
    ) -> Box<dyn DeviceContext<H::Cpu<'a>> + 'a> {
        Box::new(Context::new(self))
    }
}
