pub use self::object::*;
pub use self::page::*;

use self::phys::PhysAllocator;
use self::stats::VmStats;
use crate::config::PAGE_SIZE;
use crate::context::{config, current_thread};
use crate::dmem::Dmem;
use crate::lock::GutexGroup;
use crate::proc::Proc;
use alloc::sync::{Arc, Weak};
use core::cmp::max;
use core::fmt::Debug;
use core::sync::atomic::{AtomicUsize, Ordering};
use krt::info;
use macros::bitflag;
use thiserror::Error;

mod object;
mod page;
mod phys;
mod stats;

/// Implementation of Virtual Memory system.
pub struct Vm {
    phys: PhysAllocator,
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
    pub fn new(
        phys_avail: [u64; 61],
        ma: Option<&MemAffinity>,
        dmem: &Dmem,
    ) -> Result<Arc<Self>, VmError> {
        let phys = PhysAllocator::new(&phys_avail, ma);

        // Get initial v_page_count and v_free_count.
        let page_size = u64::try_from(PAGE_SIZE.get()).unwrap();
        let config = config();
        let blocked = config.env("vm.blacklist");
        let unk = dmem.game_end() - dmem.config().fmem_max.get();
        let mut page_count = [0; 2];
        let mut free_count = [0; 2];

        for i in (0..).step_by(2) {
            // Check if end entry.
            let mut addr = phys_avail[i];
            let end = phys_avail[i + 1];

            if end == 0 {
                break;
            }

            while addr < end {
                if blocked.is_some() {
                    todo!();
                }

                if addr < unk || dmem.game_end() <= addr {
                    // TODO: Update vm_phys_segs.
                    page_count[0] += 1;
                    free_count[0] += 1;
                } else {
                    // TODO: Update vm_phys_segs.
                    page_count[1] += 1;
                }

                addr += page_size;
            }
        }

        info!(
            concat!(
                "VM stats initialized.\n",
                "v_page_count[0]: {}\n",
                "v_free_count[0]: {}\n",
                "v_page_count[1]: {}"
            ),
            page_count[0], free_count[0], page_count[1]
        );

        // Initializes stats. The Orbis initialize these data in vm_pageout function but it is
        // possible for data race so we do it here instead.
        let pageout_page_count = 0x10; // TODO: Figure out where this value come from.
        let gg = GutexGroup::new();
        let stats = [
            VmStats {
                free_reserved: pageout_page_count + 100 + 10,
                cache_count: gg.clone().spawn_default(),
                free_count: gg.clone().spawn(free_count[0]),
                interrupt_free_min: gg.clone().spawn(2),
            },
            VmStats {
                free_reserved: pageout_page_count,
                cache_count: gg.clone().spawn_default(),
                free_count: gg.clone().spawn(free_count[1]),
                interrupt_free_min: gg.clone().spawn(2),
            },
        ];

        // Spawn page daemons. The Orbis do this in a separated sysinit but we do it here instead to
        // keep it in the VM subsystem.
        let mut vm = Self {
            phys,
            stats,
            pagers: Default::default(),
            pages_deficit: [AtomicUsize::new(0), AtomicUsize::new(0)],
        };

        vm.spawn_pagers();

        Ok(Arc::new(vm))
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

        // Allocate VmPage.
        let page = match &obj {
            Some(_) => todo!(),
            None => {
                if flags.has_any(VmAlloc::Cached) {
                    return None;
                }

                self.phys.alloc_page(vm, obj.is_none().into(), 0)
            }
        };

        // TODO: The Orbis assume page is never null here.
        let page = page.unwrap();

        match page.flags().has_any(PageFlags::Cached) {
            true => todo!(),
            false => todo!(),
        }
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

/// Implementation of `mem_affinity` structure.
pub struct MemAffinity {}

/// Flags for [`Vm::alloc_page()`].
#[bitflag(u32)]
pub enum VmAlloc {
    /// `VM_ALLOC_INTERRUPT`.
    Interrupt = 0x00000001,
    /// `VM_ALLOC_SYSTEM`.
    System = 0x00000002,
    /// `VM_ALLOC_IFCACHED`.
    Cached = 0x00000400,
    /// `VM_ALLOC_COUNT`.
    Count(u16) = 0xFFFF0000,
}

/// Represents an error when [`Vm::new()`] fails.
#[derive(Debug, Error)]
pub enum VmError {}
