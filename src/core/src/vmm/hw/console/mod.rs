use self::context::Context;
use super::{Device, DeviceContext, Ram};
use crate::vmm::VmmEventHandler;
use obvirt::console::Memory;
use std::num::NonZero;

mod context;

/// Virtual console for the VM.
pub struct Console {
    addr: usize,
    len: NonZero<usize>,
    event: VmmEventHandler,
}

impl Console {
    pub fn new(addr: usize, vm_page_size: NonZero<usize>, event: VmmEventHandler) -> Self {
        let len = size_of::<Memory>()
            .checked_next_multiple_of(vm_page_size.get())
            .and_then(NonZero::new)
            .unwrap();

        addr.checked_add(len.get()).unwrap();

        Self { addr, len, event }
    }
}

impl Device for Console {
    fn addr(&self) -> usize {
        self.addr
    }

    fn len(&self) -> NonZero<usize> {
        self.len
    }

    fn create_context<'a>(&'a self, ram: &'a Ram) -> Box<dyn DeviceContext + 'a> {
        Box::new(Context::new(self, ram))
    }
}
