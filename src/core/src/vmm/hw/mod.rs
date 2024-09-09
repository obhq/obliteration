// SPDX-License-Identifier: MIT OR Apache-2.0
use super::hv::{CpuIo, Hypervisor};
use super::VmmEventHandler;
use std::collections::BTreeMap;
use std::error::Error;
use std::num::NonZero;
use std::sync::Arc;

pub use self::console::*;

mod console;

pub fn setup_devices<H: Hypervisor>(
    start_addr: usize,
    block_size: NonZero<usize>,
    event: VmmEventHandler,
) -> DeviceTree<H> {
    let mut map = BTreeMap::<usize, Arc<dyn Device<H>>>::new();

    // Console.
    let addr = start_addr;
    let console = Arc::new(Console::new(addr, block_size, event));

    assert!(map
        .insert(<Console as Device<H>>::addr(&console), console.clone())
        .is_none());

    // Make sure nothing are overlapped.
    let mut end = start_addr;

    for (addr, dev) in &map {
        assert!(*addr >= end);
        end = addr.checked_add(dev.len().get()).unwrap();
    }

    DeviceTree { console, map }
}

/// Contains all virtual devices, except RAM; for the VM.
pub struct DeviceTree<H: Hypervisor> {
    console: Arc<Console>,
    map: BTreeMap<usize, Arc<dyn Device<H>>>,
}

impl<H: Hypervisor> DeviceTree<H> {
    pub fn console(&self) -> &impl Device<H> {
        self.console.as_ref()
    }

    /// Returns iterator ordered by physical address.
    pub fn map(&self) -> impl Iterator<Item = (usize, &dyn Device<H>)> + '_ {
        self.map.iter().map(|(addr, dev)| (*addr, dev.as_ref()))
    }
}

/// Virtual device that has a physical address in the virtual machine.
pub trait Device<H: Hypervisor>: Send + Sync {
    /// Physical address in the virtual machine.
    fn addr(&self) -> usize;

    /// Total size of device memory, in bytes.
    fn len(&self) -> NonZero<usize>;

    fn create_context<'a>(&'a self, hv: &'a H) -> Box<dyn DeviceContext + 'a>;
}

/// Context to execute memory-mapped I/O operations on a virtual device.
pub trait DeviceContext {
    fn exec(&mut self, exit: &mut dyn CpuIo) -> Result<bool, Box<dyn Error>>;
}
