// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::console::*;
pub use self::vmm::*;

use crate::hv::{Cpu, CpuExit, CpuIo, Hypervisor, IoBuf, LockedAddr};
use std::collections::BTreeMap;
use std::error::Error;
use std::num::NonZero;
use std::sync::Arc;
use thiserror::Error;

mod console;
mod vmm;

pub fn setup_devices(start_addr: usize, block_size: NonZero<usize>) -> DeviceTree {
    let mut b = MapBuilder {
        map: BTreeMap::new(),
        next: start_addr,
    };

    let vmm = b.push(|addr| Vmm::new(addr, block_size));
    let console = b.push(|addr| Console::new(addr, block_size));

    DeviceTree {
        vmm,
        console,
        map: b.map,
    }
}

fn read_u8(exit: &mut impl CpuIo) -> Result<u8, MmioError> {
    match exit.buffer() {
        IoBuf::Write(&[v]) => Ok(v),
        IoBuf::Write(_) => Err(MmioError::InvalidData),
        _ => Err(MmioError::InvalidOperation),
    }
}

fn read_usize(exit: &mut impl CpuIo) -> Result<usize, MmioError> {
    // Get data.
    let IoBuf::Write(data) = exit.buffer() else {
        return Err(MmioError::InvalidOperation);
    };

    // Parse data.
    data.try_into()
        .map(usize::from_ne_bytes)
        .map_err(|_| MmioError::InvalidData)
}

fn read_ptr<'a>(
    exit: &mut impl CpuIo,
    len: NonZero<usize>,
    hv: &'a impl Hypervisor,
) -> Result<LockedAddr<'a>, MmioError> {
    // Get data.
    let IoBuf::Write(buf) = exit.buffer() else {
        return Err(MmioError::InvalidOperation);
    };

    // Get address.
    let vaddr = buf
        .try_into()
        .map(usize::from_ne_bytes)
        .map_err(|_| MmioError::InvalidData)?;

    let paddr = exit
        .cpu()
        .translate(vaddr)
        .map_err(|e| MmioError::TranslateVaddrFailed(vaddr, Box::new(e)))?;

    // Get data.
    hv.ram()
        .lock(paddr, len)
        .ok_or(MmioError::InvalidAddr { vaddr, paddr })
}

/// Contains all virtual devices (except RAM) for the VM.
///
/// All devices guarantee to not overlapped.
pub struct DeviceTree {
    vmm: Arc<Vmm>,
    console: Arc<Console>,
    map: BTreeMap<usize, Arc<dyn Device>>,
}

impl DeviceTree {
    pub fn vmm(&self) -> &Vmm {
        self.vmm.as_ref()
    }

    pub fn console(&self) -> &Console {
        self.console.as_ref()
    }

    /// Returns iterator ordered by physical address.
    pub fn all(&self) -> impl Iterator<Item = (usize, &dyn Device)> + '_ {
        self.map.iter().map(|(addr, dev)| (*addr, dev.as_ref()))
    }
}

/// Virtual device that has a physical address in the virtual machine.
pub trait Device: Send + Sync {
    /// Display name of this device.
    fn name(&self) -> &str;

    /// Physical address in the virtual machine.
    fn addr(&self) -> usize;

    /// Total size of device memory, in bytes.
    fn len(&self) -> NonZero<usize>;
}

/// Context for a CPU to execute operations on a virtual device.
pub trait DeviceContext<C: Cpu> {
    /// Execute immeditately after the VM exited.
    fn exited(&mut self, cpu: &mut C) -> Result<bool, Box<dyn Error>> {
        let _ = cpu;
        Ok(true)
    }

    /// Execute only if the CPU read or write into this device address.
    fn mmio(&mut self, exit: &mut <C::Exit<'_> as CpuExit>::Io) -> Result<bool, Box<dyn Error>>;

    /// Always execute after the exited event has been handled (before enter the VM again).
    fn post(&mut self, cpu: &mut C) -> Result<bool, Box<dyn Error>> {
        let _ = cpu;
        Ok(true)
    }
}

/// Struct to build a map of virtual device.
struct MapBuilder {
    map: BTreeMap<usize, Arc<dyn Device>>,
    next: usize,
}

impl MapBuilder {
    fn push<T: Device + 'static>(&mut self, f: impl FnOnce(usize) -> T) -> Arc<T> {
        let d = Arc::new(f(self.next));

        assert!(self.map.insert(self.next, d.clone()).is_none());
        self.next = self.next.checked_add(d.len().get()).unwrap();

        d
    }
}

/// Represents an error when a Memory-mapped I/O operation fails.
#[derive(Debug, Error)]
enum MmioError {
    #[error("invalid operation")]
    InvalidOperation,

    #[error("invalid data")]
    InvalidData,

    #[error("couldn't translate {0:#x} to physical address")]
    TranslateVaddrFailed(usize, #[source] Box<dyn Error>),

    #[error("address {vaddr:#x} ({paddr:#x}) is not allocated")]
    InvalidAddr { vaddr: usize, paddr: usize },
}
