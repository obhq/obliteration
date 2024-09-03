use super::VmmEventHandler;
use crate::vmm::hv::CpuIo;
use std::collections::BTreeMap;
use std::error::Error;
use std::num::NonZero;
use std::sync::Arc;

pub use self::console::*;
pub use self::ram::*;

mod console;
mod ram;

pub fn setup_devices(
    start_addr: usize,
    block_size: NonZero<usize>,
    event: VmmEventHandler,
) -> DeviceTree {
    let mut map = BTreeMap::<usize, Arc<dyn Device>>::new();

    // Console.
    let addr = start_addr;
    let console = Arc::new(Console::new(addr, block_size, event));

    assert!(map.insert(console.addr(), console.clone()).is_none());

    // Make sure nothing are overlapped.
    let mut end = start_addr;

    for (addr, dev) in &map {
        assert!(*addr >= end);
        end = addr.checked_add(dev.len().get()).unwrap();
    }

    DeviceTree { console, map }
}

/// Contains all virtual devices, except RAM; for the VM.
pub struct DeviceTree {
    console: Arc<Console>,
    map: BTreeMap<usize, Arc<dyn Device>>,
}

impl DeviceTree {
    pub fn console(&self) -> &Console {
        &self.console
    }

    /// Returns iterator ordered by physical address.
    pub fn map(&self) -> impl Iterator<Item = (usize, &dyn Device)> + '_ {
        self.map.iter().map(|(addr, dev)| (*addr, dev.as_ref()))
    }
}

/// Virtual device that has a physical address in the virtual machine.
pub trait Device: Send + Sync {
    /// Physical address in the virtual machine.
    fn addr(&self) -> usize;

    /// Total size of device memory, in bytes.
    fn len(&self) -> NonZero<usize>;

    fn create_context<'a>(&'a self, ram: &'a Ram) -> Box<dyn DeviceContext + 'a>;
}

/// Context to execute memory-mapped I/O operations on a virtual device.
pub trait DeviceContext {
    fn exec(&mut self, exit: &mut dyn CpuIo) -> Result<bool, Box<dyn Error>>;
}
