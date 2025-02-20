pub use self::object::*;

use self::stats::VmStats;
use crate::lock::GutexGroup;
use alloc::sync::Arc;

mod object;
mod stats;

/// Implementation of Virtual Memory system.
pub struct Vm {
    stats: [VmStats; 3],
}

impl Vm {
    pub fn new() -> Arc<Self> {
        // Initializes stats. The Orbis initialize these data in vm_pageout function but it is
        // possible for data race.
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

        Arc::new(Self { stats })
    }

    /// See `vm_page_alloc` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x02B030|
    pub fn alloc_page(&self, obj: Option<VmObject>) {
        // Get target VM.
        let vm = match obj {
            Some(_) => todo!(),
            None => 0,
        };

        let stats = &self.stats[vm];
        let cache_count = stats.cache_count.read();
        let free_count = stats.free_count.read();

        if *cache_count + *free_count <= stats.free_reserved {
            todo!()
        }

        todo!()
    }
}
