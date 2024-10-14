// SPDX-License-Identifier: MIT OR Apache-2.0
use self::context::Context;
use super::{Device, DeviceContext};
use crate::vmm::cpu::DebugStates;
use crate::vmm::hv::Cpu;
use crate::vmm::VmmEventHandler;
use obconf::DebuggerMemory;
use std::num::NonZero;
use std::sync::{Condvar, Mutex};

mod context;

/// Virtual device for the kernel to communicate with the debugger.
pub struct Debugger {
    addr: usize,
    len: NonZero<usize>,
    event: VmmEventHandler,
}

impl Debugger {
    pub fn new(addr: usize, block_size: NonZero<usize>, event: VmmEventHandler) -> Self {
        let len = size_of::<DebuggerMemory>()
            .checked_next_multiple_of(block_size.get())
            .and_then(NonZero::new)
            .unwrap();

        Self { addr, len, event }
    }

    pub fn create_context<'a, C: Cpu>(
        &'a self,
        debug: &'a (Mutex<DebugStates>, Condvar),
    ) -> Box<dyn DeviceContext<C> + 'a> {
        Box::new(Context::new(self, debug))
    }
}

impl Device for Debugger {
    fn name(&self) -> &str {
        "Debugger"
    }

    fn addr(&self) -> usize {
        self.addr
    }

    fn len(&self) -> NonZero<usize> {
        self.len
    }
}
