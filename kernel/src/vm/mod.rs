pub use self::object::*;
pub use self::page::*;

use self::stats::VmStats;
use crate::MemoryInfo;
use crate::context::current_thread;
use crate::lock::GutexGroup;
use crate::proc::Proc;
use alloc::sync::{Arc, Weak};
use core::cmp::max;
use core::fmt::Debug;
use core::sync::atomic::{AtomicUsize, Ordering};
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
    ///
    /// # Safety
    /// All fields in `mi` must be correct.
    pub unsafe fn new(mi: &MemoryInfo) -> Result<Arc<Self>, VmError> {
        // Initializes stats. The Orbis initialize these data in vm_pageout function but it is
        // possible for data race so we do it here instead.
        let pageout_page_count = 0x10; // TODO: Figure out where this value come from.
        let gg = GutexGroup::new();
        let stats = [
            VmStats {
                free_reserved: pageout_page_count + 100 + 10,
                cache_count: gg.clone().spawn_default(),
                free_count: gg.clone().spawn_default(),
                interrupt_free_min: gg.clone().spawn(2),
            },
            VmStats {
                free_reserved: pageout_page_count,
                cache_count: gg.clone().spawn_default(),
                free_count: gg.clone().spawn_default(),
                interrupt_free_min: gg.clone().spawn(2),
            },
        ];

        // Spawn page daemons. The Orbis do this in a separated sysinit but we do it here instead to
        // keep it in the VM subsystem.
        let mut vm = Self {
            boot_area: mi.boot_area,
            boot_addr: mi.boot_info.addr,
            boot_tables: mi.boot_info.page_tables,
            initial_memory_size: mi.initial_memory_size,
            end_page: mi.end_page,
            stats,
            pagers: Default::default(),
            pages_deficit: [AtomicUsize::new(0), AtomicUsize::new(0)],
        };

        vm.spawn_pagers();

        Ok(Arc::new(vm))
    }

    pub fn boot_area(&self) -> u64 {
        self.boot_area
    }

    pub fn boot_addr(&self) -> u64 {
        self.boot_addr
    }

    pub fn boot_tables(&self) -> u64 {
        self.boot_tables
    }

    pub fn initial_memory_size(&self) -> u64 {
        self.initial_memory_size
    }

    pub fn end_page(&self) -> u64 {
        self.end_page
    }

    /// See `vm_page_alloc` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x02B030|
    pub fn alloc_page(&self, obj: Option<VmObject>, flags: VmAlloc) -> Option<VmPage> {
        let vm = obj.as_ref().map_or(0, |v| v.vm());
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
pub enum VmError {}
