pub use self::object::*;

use self::stats::VmStats;
use crate::config::{PAGE_MASK, PAGE_SIZE};
use crate::context::current_thread;
use crate::lock::GutexGroup;
use crate::proc::Proc;
use alloc::sync::{Arc, Weak};
use config::{BootEnv, MapType};
use krt::{boot_env, warn};
use macros::bitflag;
use thiserror::Error;

mod object;
mod stats;

/// Implementation of Virtual Memory system.
pub struct Vm {
    boot_area: u64, // basemem
    initial_memory_size: u64,
    stats: [VmStats; 3],
    pagers: [Weak<Proc>; 2], // pageproc
}

impl Vm {
    /// See `vm_page_startup` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x029200|
    pub fn new() -> Result<Arc<Self>, VmError> {
        // Initializes stats. The Orbis initialize these data in vm_pageout function but it is
        // possible for data race so we do it here instead.
        let pageout_page_count = 0x10; // TODO: Figure out where this value come from.
        let gg = GutexGroup::new();
        let stats = [
            VmStats {
                free_reserved: pageout_page_count + 100 + 10, // TODO: Same here.
                cache_count: gg.clone().spawn_default(),
                free_count: gg.clone().spawn_default(),
            },
            VmStats {
                #[allow(clippy::identity_op)]
                free_reserved: pageout_page_count + 0, // TODO: Same here.
                cache_count: gg.clone().spawn_default(),
                free_count: gg.clone().spawn_default(),
            },
            VmStats {
                free_reserved: 0,
                cache_count: gg.clone().spawn_default(),
                free_count: gg.spawn_default(),
            },
        ];

        // The Orbis invoke this in hammer_time but we do it here instead to keep it in the VM
        // subsystem.
        let mut vm = Self {
            boot_area: 0,
            initial_memory_size: 0,
            stats,
            pagers: Default::default(),
        };

        vm.load_memory_map()?;

        // Spawn page daemons. The Orbis do this in a separated sysinit but we do it here instead to
        // keep it in the VM subsystem.
        vm.spawn_pagers();

        Ok(Arc::new(vm))
    }

    pub fn boot_area(&self) -> u64 {
        self.boot_area
    }

    pub fn initial_memory_size(&self) -> u64 {
        self.initial_memory_size
    }

    /// See `vm_page_alloc` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x02B030|
    pub fn alloc_page(&self, obj: Option<VmObject>, _: VmAlloc) {
        // Get target VM.
        let vm = match obj {
            Some(_) => todo!(),
            None => 0,
        };

        let td = current_thread();
        let stats = &self.stats[vm];
        let cache_count = stats.cache_count.read();
        let free_count = stats.free_count.read();

        if *cache_count + *free_count <= stats.free_reserved {
            // Page daemon should never die so we use unwrap to catch that here.
            let p = td.proc();

            if Arc::ptr_eq(p, &self.pagers[p.pager()].upgrade().unwrap()) {
                todo!()
            } else {
                todo!()
            }
        }

        todo!()
    }

    /// See `getmemsize` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x25CF00|
    fn load_memory_map(&mut self) -> Result<(), VmError> {
        // TODO: Some of the logic around physmap does not make sense.
        let mut physmap = [0u64; 60];
        let mut i = 0usize;
        let map = match boot_env() {
            BootEnv::Vm(v) => v.memory_map.as_slice(),
        };

        'top: for m in map {
            // We only interested in RAM.
            match m.ty {
                MapType::None => break,
                MapType::Ram => (),
                MapType::Reserved => continue,
            }

            // TODO: This should be possible only when booting from BIOS.
            if m.len == 0 {
                break;
            }

            let mut insert_idx = i + 2;
            let mut j = 0usize;

            while j <= i {
                if m.base < physmap[j + 1] {
                    if physmap[j] < m.base + m.len {
                        warn!("Overlapping memory regions, ignoring second region.");
                        continue 'top;
                    }

                    insert_idx = j;
                    break;
                }

                j += 2;
            }

            if insert_idx <= i && m.base + m.len == physmap[insert_idx] {
                physmap[insert_idx] = m.base;
                continue;
            }

            if insert_idx > 0 && m.base == physmap[insert_idx - 1] {
                physmap[insert_idx - 1] = m.base + m.len;
                continue;
            }

            i += 2;

            if i == physmap.len() {
                warn!("Too many segments in the physical address map, giving up.");
                break;
            }

            // This loop does not make sense on the Orbis. It seems like if this loop once
            // entered it will never exit.
            #[allow(clippy::while_immutable_condition)]
            while insert_idx < i {
                todo!()
            }

            physmap[insert_idx] = m.base;
            physmap[insert_idx + 1] = m.base + m.len;
        }

        // Check if bootloader provide us a memory map.
        let physmap = &physmap[..i];

        if physmap.is_empty() {
            return Err(VmError::NoMemoryMap);
        }

        // Get initial memory size and BIOS boot area.
        let page_size = PAGE_SIZE.get().try_into().unwrap();
        let page_mask = !u64::try_from(PAGE_MASK.get()).unwrap();

        for e in physmap.chunks(2) {
            // Check if BIOS boot area.
            if e[0] == 0 {
                // TODO: Why 1024?
                self.boot_area = e[1] / 1024;
            }

            // Add to initial memory size.
            let start = e[0].next_multiple_of(page_size);
            let end = e[1] & page_mask;

            self.initial_memory_size += end.saturating_sub(start);
        }

        if self.boot_area == 0 {
            return Err(VmError::NoBootArea);
        }

        Ok(())
    }

    /// See `kick_pagedaemons` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x3E0E40|
    fn spawn_pagers(&mut self) {
        todo!()
    }
}

/// Flags for [`Vm::alloc_page()`].
#[bitflag(u32)]
pub enum VmAlloc {}

/// Represents an error when [`Vm::new()`] fails.
#[derive(Debug, Error)]
pub enum VmError {
    #[error("no memory map provided to the kernel")]
    NoMemoryMap,

    #[error("no boot area provided to the kernel")]
    NoBootArea,
}
