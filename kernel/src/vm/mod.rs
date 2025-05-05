pub use self::object::*;
pub use self::page::*;

use self::stats::VmStats;
use crate::config::{Dipsw, PAGE_MASK, PAGE_SHIFT, PAGE_SIZE};
use crate::context::{current_arch, current_config, current_thread};
use crate::lock::GutexGroup;
use crate::proc::Proc;
use alloc::sync::{Arc, Weak};
use config::{BootEnv, MapType};
use core::cmp::{max, min};
use core::fmt::Debug;
use core::sync::atomic::{AtomicUsize, Ordering};
use krt::{boot_env, warn};
use macros::bitflag;
use thiserror::Error;

mod object;
mod page;
mod stats;

/// Implementation of Virtual Memory system.
pub struct Vm {
    boot_area: u64,           // basemem
    boot_addr: u64,           // boot_address
    boot_tables: u64,         // mptramp_pagetables
    initial_memory_size: u64, // initial_memory_size
    end_page: u64,            // Maxmem
    stats: [VmStats; 2],
    pagers: [Weak<Proc>; 2],         // pageproc
    pages_deficit: [AtomicUsize; 2], // vm_pageout_deficit
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
                interrupt_free_min: gg.clone().spawn(2),
            },
            VmStats {
                #[allow(clippy::identity_op)]
                free_reserved: pageout_page_count + 0, // TODO: Same here.
                cache_count: gg.clone().spawn_default(),
                free_count: gg.clone().spawn_default(),
                interrupt_free_min: gg.clone().spawn(2),
            },
        ];

        // The Orbis invoke this in hammer_time but we do it here instead to keep it in the VM
        // subsystem.
        let mut vm = Self {
            boot_area: 0,
            boot_addr: 0,
            boot_tables: 0,
            initial_memory_size: 0,
            end_page: 0,
            stats,
            pagers: Default::default(),
            pages_deficit: [AtomicUsize::new(0), AtomicUsize::new(0)],
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
    pub fn alloc_page(&self, obj: Option<VmObject>, flags: VmAlloc) -> Option<VmPage> {
        let vm = obj.as_ref().map(|v| v.vm()).unwrap_or(0);
        let td = current_thread();
        let stats = &self.stats[vm];
        let cache_count = stats.cache_count.read();
        let free_count = stats.free_count.read();
        let available = *free_count + *cache_count;

        if available <= stats.free_reserved {
            let p = td.proc();
            let mut flags = if Arc::as_ptr(p) == self.pagers[p.pager()].as_ptr() {
                VmAlloc::System.into()
            } else {
                flags & (VmAlloc::Interrupt | VmAlloc::System)
            };

            if (flags & (VmAlloc::Interrupt | VmAlloc::System)) == VmAlloc::Interrupt {
                flags = VmAlloc::Interrupt.into();
            }

            if flags == VmAlloc::Interrupt {
                todo!()
            } else if flags == VmAlloc::System {
                if available <= *stats.interrupt_free_min.read() {
                    let deficit = max(1, flags.get(VmAlloc::Count));

                    drop(free_count);
                    drop(cache_count);

                    self.pages_deficit[vm].fetch_add(deficit.into(), Ordering::Relaxed);
                    self.wake_pager(vm);

                    return None;
                }
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
        // TODO: Some of the logic around here are very hard to understand.
        let mut physmap = [0u64; 60];
        let mut last = 0usize;
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

            // Check if we need to insert before the previous entries.
            let mut insert_idx = last + 2;
            let mut j = 0usize;

            while j <= last {
                if m.base < physmap[j + 1] {
                    // Check if end address overlapped.
                    if m.base + m.len > physmap[j] {
                        warn!("Overlapping memory regions, ignoring second region.");
                        continue 'top;
                    }

                    insert_idx = j;
                    break;
                }

                j += 2;
            }

            // Check if end address is the start address of the next entry. If yes we just change
            // base address of it to increase its size.
            if insert_idx <= last && m.base + m.len == physmap[insert_idx] {
                physmap[insert_idx] = m.base;
                continue;
            }

            // Check if start address is the end address of the previous entry. If yes we just
            // increase the size of previous entry.
            if insert_idx > 0 && m.base == physmap[insert_idx - 1] {
                physmap[insert_idx - 1] = m.base + m.len;
                continue;
            }

            last += 2;

            if last == physmap.len() {
                warn!("Too many segments in the physical address map, giving up.");
                break;
            }

            // This loop does not make sense on the Orbis. It seems like if this loop once
            // entered it will never exit.
            #[allow(clippy::while_immutable_condition)]
            while insert_idx < last {
                todo!()
            }

            physmap[insert_idx] = m.base;
            physmap[insert_idx + 1] = m.base + m.len;
        }

        // Check if bootloader provide us a memory map. The Orbis will check if
        // preload_search_info() return null but we can't do that since we use a static size array
        // to pass this information.
        if physmap[1] == 0 {
            return Err(VmError::NoMemoryMap);
        }

        // Get initial memory size and BIOS boot area.
        let page_size = PAGE_SIZE.get().try_into().unwrap();
        let page_mask = !u64::try_from(PAGE_MASK.get()).unwrap();

        for i in (0..=last).step_by(2) {
            // Check if BIOS boot area.
            if physmap[i] == 0 {
                // TODO: Why 1024?
                self.boot_area = physmap[i + 1] / 1024;
            }

            // Add to initial memory size.
            let start = physmap[i].next_multiple_of(page_size);
            let end = physmap[i + 1] & page_mask;

            self.initial_memory_size += end.saturating_sub(start);
        }

        if self.boot_area == 0 {
            return Err(VmError::NoBootArea);
        }

        // TODO: This seems like it is assume the first physmap always a boot area. The problem is
        // what is the point of the logic on the above to find boot_area?
        physmap[1] = self.adjust_boot_area(physmap[1] / 1024);

        // Get end page.
        self.end_page = physmap[last + 1] >> PAGE_SHIFT;

        if let Some(v) = current_config().env("hw.physmem") {
            self.end_page = min(v.parse::<u64>().unwrap() >> PAGE_SHIFT, self.end_page);
        }

        // TODO: There is some unknown calls here.
        self.load_pmap();

        Ok(())
    }

    /// See `kick_pagedaemons` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x3E0E40|
    fn spawn_pagers(&mut self) {
        // TODO: This requires v_page_count that populated by vm_page_startup. In order to populate
        // this we need phys_avail that populated by getmemsize.
    }

    /// See `pagedaemon_wakeup` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x3E0690|
    fn wake_pager(&self, _: usize) {
        todo!()
    }

    /// See `mp_bootaddress` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x1B9D20|
    fn adjust_boot_area(&mut self, original: u64) -> u64 {
        // TODO: Most logic here does not make sense.
        let page_size = u64::try_from(PAGE_SIZE.get()).unwrap();
        let page_mask = !u64::try_from(PAGE_MASK.get()).unwrap();
        let need = u64::try_from(current_arch().secondary_start.len()).unwrap();
        let addr = (original * 1024) & page_mask;

        // TODO: What is this?
        self.boot_addr = if need <= ((original * 1024) & 0xC00) {
            addr
        } else {
            addr - page_size
        };

        self.boot_tables = self.boot_addr - (page_size * 3);
        self.boot_tables
    }

    /// See `pmap_bootstrap` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x1127C0|
    fn load_pmap(&mut self) {
        let config = current_config();

        if config.is_allow_disabling_aslr() && config.dipsw(Dipsw::DisabledKaslr) {
            todo!()
        } else {
            // TODO: There are a lot of unknown variables here so we skip implementing this until we
            // run into the code that using them.
        }
    }
}

/// Flags for [`Vm::alloc_page()`].
#[bitflag(u32)]
pub enum VmAlloc {
    /// `VM_ALLOC_INTERRUPT`.
    Interrupt = 0x00000001,
    /// `VM_ALLOC_SYSTEM`.
    System = 0x00000002,
    /// `VM_ALLOC_COUNT`.
    Count(u16) = 0xFFFF0000,
}

/// Represents an error when [`Vm::new()`] fails.
#[derive(Debug, Error)]
pub enum VmError {
    #[error("no memory map provided to the kernel")]
    NoMemoryMap,

    #[error("no boot area provided to the kernel")]
    NoBootArea,
}
