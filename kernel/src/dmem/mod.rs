use crate::MemoryInfo;
use crate::config::{Dipsw, PAGE_SHIFT};
use crate::context::config;
use alloc::sync::Arc;
use core::num::NonZero;

/// Implementation of Direct Memory system.
pub struct Dmem {
    mode: usize,
    config: &'static DmemConfig,
}

impl Dmem {
    /// See `initialize_dmem` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x3F5C20|
    pub fn new(mi: &mut MemoryInfo) -> Arc<Self> {
        // TODO: Figure out what the purpose of this 16MB block of memory.
        let haddr = Self::reserve_phys(mi, 0x1000000.try_into().unwrap(), 1);

        if haddr == 0 {
            panic!("no available memory for high-address memory");
        }

        // TODO: Invoke bootparam_get_ddr3_capacity.
        let mode = Self::load_mode(mi);

        if (0x80C6C0C4u64 & (1 << mode)) != 0 {
            panic!("game DMEM size is not configured");
        }

        // Allocate game DMEM.
        let dc = DMEM_CONFIGS[mode].as_ref().unwrap();
        let game = Self::reserve_phys(mi, dc.game_size, 0x8000000);

        if game == 0 {
            panic!("not enough memory for game DMEM");
        }

        // TODO: There is an unknown call here.
        let game_end = game + dc.game_size.get();

        if (0x7F393733u64 & (1 << mode)) != 0 {
            // Get alignment for mini-app DMEM.
            let align = if config().unknown_dmem1() == 0 {
                0x8000000i64
            } else {
                0x200000i64
            };

            // Allocate mini-app DMEM.
            let size = dc.mini_size;
            let mini = match dc.mini_shared {
                true => (-align) as u64 & (game_end - (dc.fmem_max.get() + size)),
                false => todo!(),
            };

            if mini == 0 {
                panic!("not enough memory for mini-app DMEM");
            }

            // TODO: Invoke pmap_change_attr.
        }

        if (0x7F393F3Bu64 & (1 << mode)) != 0 {
            let size = dc.vsh_size;
            let vsh = Self::reserve_phys(mi, size, 0x200000);

            if vsh == 0 {
                panic!("not enough memory for VSH DMEM");
            }

            // TODO: There are some write to unknow variable here.
            // TODO: Invoke pmap_change_attr.
        }

        if (0x47000703u64 & (1 << mode)) != 0 {
            todo!()
        }

        // TODO: There is a write to unknown variable here.
        if (0x47010703u64 & (1 << mode)) != 0 {
            todo!()
        }

        // Allocate vision DMEM.
        let vision = Self::reserve_phys(mi, 0x1000000.try_into().unwrap(), 0x200000);

        if vision == 0 {
            panic!("not enough memory for vision DMEM");
        }

        // TODO: Invoke pmap_change_attr.
        // TODO: There are some write to unknown variables here.
        let mut unk = dc.unk1;

        if ((0x80C6C0C4u64 + 0x20212022u64) & (1 << mode)) == 0 {
            loop {
                let (size, align) = if unk < 0x200000 {
                    // TODO: Verify if 0x4000 is actually a (virtual) page size.
                    (unk, 0x4000)
                } else {
                    (0x200000, 0x200000)
                };

                // TODO: Invoke pmap_change_attr.
                let addr = Self::reserve_phys(mi, size.try_into().unwrap(), align);

                if addr == 0 {
                    // TODO: Figure out the name of this memory.
                    panic!("failed to reserve memory");
                }

                // Check if completed.
                unk -= size;

                if unk == 0 {
                    break;
                }
            }
        }

        Self::adjust_pmap(mi, game_end - dc.fmem_max.get(), game_end);
        Self::adjust_pmap(
            mi,
            haddr,
            haddr + 0xC00000 + if mi.unk != 0 { 0x400000 } else { 0 },
        );

        mi.end_page = mi.physmap[mi.physmap_last + 1] >> PAGE_SHIFT;

        Arc::new(Self { mode, config: dc })
    }

    pub fn mode(&self) -> usize {
        self.mode
    }

    pub fn config(&self) -> &'static DmemConfig {
        self.config
    }

    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x3F64C0|
    fn reserve_phys(mi: &mut MemoryInfo, size: NonZero<u64>, align: i64) -> u64 {
        let mut i = mi.physmap_last;

        loop {
            let start = mi.physmap[i];
            let end = mi.physmap[i + 1];
            let addr = (end - size.get()) & ((-align) as u64);

            if addr >= start {
                let aligned_end = addr + size.get();

                // Check if this take the whole block.
                if (addr == start) && (aligned_end == end) {
                    mi.physmap.copy_within((i + 2).., i);
                    mi.physmap_last -= 2;
                    return addr;
                }

                // Check if this create a hole in the block.
                if (addr != start) && (aligned_end != end) {
                    mi.physmap.copy_within(i..(mi.physmap_last + 2), i + 2);
                    mi.physmap[i + 1] = addr;
                    mi.physmap[i + 2] = aligned_end;
                    mi.physmap_last += 2;
                    return addr;
                }

                // Check if this take the end of the block.
                if addr != start {
                    assert_eq!(aligned_end, end);
                    mi.physmap[i + 1] = addr;
                    return addr;
                }

                // Take the start of the block.
                mi.physmap[i] = aligned_end;

                return addr;
            }

            // Move to lower map.
            if i < 2 {
                break 0;
            }

            i -= 2;
        }
    }

    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x3F65B0|
    fn adjust_pmap(mi: &mut MemoryInfo, start: u64, end: u64) {
        // Orbis also check if physmap_last + 2 is less than one. We don't do this because it is
        // impossible for the value to be zero.
        let mut idx = 0;

        if start > mi.physmap[0] {
            // Find the first map with address greater or equal start address.
            let mut found = true;

            for i in (2..).step_by(2) {
                idx = i;

                if idx > mi.physmap_last {
                    found = false;
                    break;
                }

                if mi.physmap[idx] >= start {
                    break;
                }
            }

            // TODO: What is this?
            let mut empty = false;

            if found {
                empty = mi.physmap[idx] == end;

                if (mi.physmap[idx - 1] == start) && (mi.physmap[idx] == end) {
                    todo!()
                }
            }

            if mi.physmap[idx - 1] == start {
                todo!()
            }

            if !empty {
                if !found {
                    mi.physmap[idx] = start;
                    mi.physmap[idx | 1] = end;
                    mi.physmap_last += 2;
                    return;
                }

                todo!()
            }
        } else {
            todo!()
        }

        mi.physmap[idx] = start;
    }

    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x3F5B10|
    fn load_mode(mi: &MemoryInfo) -> usize {
        // Ths PS4 cache the calculation of this value here but we move it to Dmem struct instead.
        let c = config();
        let v = if c.unknown_dmem1() == 0 {
            if c.dipsw(Dipsw::Unk97) {
                // TODO: Figure out the name of this constant.
                3
            } else if c.dipsw(Dipsw::Unk0) && !c.dipsw(Dipsw::Unk24) {
                // TODO: Figure out the name of 3 constant.
                if c.dipsw(Dipsw::Unk16) && c.dipsw(Dipsw::Unk17) && (mi.unk & 3) == 3 {
                    // TODO: Figure out the name of this constant.
                    6
                } else {
                    todo!()
                }
            } else {
                // TODO: Figure out the name of this constant.
                4
            }
        } else {
            // TODO: Figure out the name of this constant.
            5
        };

        v + usize::try_from(mi.unk).unwrap() * 8
    }
}

/// Configurations set for each DMEM mode.
pub struct DmemConfig {
    pub name: &'static str,
    pub game_size: NonZero<u64>,
    pub fmem_max: NonZero<u64>,
    pub mini_size: u64,
    pub mini_shared: bool,
    pub vsh_size: NonZero<u64>,
    pub unk1: u64,
}

// TODO: It is likely to be more than 21 entries on PS4 11.00.
static DMEM_CONFIGS: [Option<DmemConfig>; 21] = [
    Some(DmemConfig {
        name: "BC8 normal",
        game_size: NonZero::new(0x148000000).unwrap(),
        fmem_max: NonZero::new(0x40000000).unwrap(),
        mini_size: 0x30000000,
        mini_shared: true,
        vsh_size: NonZero::new(0x17C00000).unwrap(),
        unk1: 0x13600000,
    }),
    Some(DmemConfig {
        name: "BC8 large",
        game_size: NonZero::new(0x170000000).unwrap(),
        fmem_max: NonZero::new(0x40000000).unwrap(),
        mini_size: 0x30000000,
        mini_shared: true,
        vsh_size: NonZero::new(0x16C00000).unwrap(),
        unk1: 0,
    }),
    None,
    Some(DmemConfig {
        name: "BC8 kratos",
        game_size: NonZero::new(0x148000000).unwrap(),
        fmem_max: NonZero::new(0x40000000).unwrap(),
        mini_size: 0,
        mini_shared: false,
        vsh_size: NonZero::new(0x1CA00000).unwrap(),
        unk1: 0xA00000,
    }),
    Some(DmemConfig {
        name: "BC8 release",
        game_size: NonZero::new(0x148000000).unwrap(),
        fmem_max: NonZero::new(0x40000000).unwrap(),
        mini_size: 0x30000000,
        mini_shared: false,
        vsh_size: NonZero::new(0x1A800000).unwrap(),
        unk1: 0xA00000,
    }),
    Some(DmemConfig {
        name: "BC8 CS",
        game_size: NonZero::new(0x124000000).unwrap(),
        fmem_max: NonZero::new(0x4000000).unwrap(),
        mini_size: 0x58800000,
        mini_shared: false,
        vsh_size: NonZero::new(0x28200000).unwrap(),
        unk1: 0,
    }),
    None,
    None,
    Some(DmemConfig {
        name: "BC16 normal",
        game_size: NonZero::new(0x148000000).unwrap(),
        fmem_max: NonZero::new(0x40000000).unwrap(),
        mini_size: 0x30000000,
        mini_shared: false,
        vsh_size: NonZero::new(0x1C200000).unwrap(),
        unk1: 0x17FB00000,
    }),
    Some(DmemConfig {
        name: "BC16 large",
        game_size: NonZero::new(0x28C000000).unwrap(),
        fmem_max: NonZero::new(0x5C000000).unwrap(),
        mini_size: 0x30000000,
        mini_shared: false,
        vsh_size: NonZero::new(0x1B200000).unwrap(),
        unk1: 0x3CB00000,
    }),
    Some(DmemConfig {
        name: "BC16 mini-app large",
        game_size: NonZero::new(0x148000000).unwrap(),
        fmem_max: NonZero::new(0x40000000).unwrap(),
        mini_size: 0x48000000,
        mini_shared: false,
        vsh_size: NonZero::new(0x19600000).unwrap(),
        unk1: 0x13A700000,
    }),
    Some(DmemConfig {
        name: "BC16 kratos",
        game_size: NonZero::new(0x148000000).unwrap(),
        fmem_max: NonZero::new(0x40000000).unwrap(),
        mini_size: 0,
        mini_shared: false,
        vsh_size: NonZero::new(0x1CA00000).unwrap(),
        unk1: 0x1F3C00000,
    }),
    Some(DmemConfig {
        name: "BC16 release",
        game_size: NonZero::new(0x148000000).unwrap(),
        fmem_max: NonZero::new(0x40000000).unwrap(),
        mini_size: 0x30000000,
        mini_shared: false,
        vsh_size: NonZero::new(0x1C200000).unwrap(),
        unk1: 0x1FF600000,
    }),
    Some(DmemConfig {
        name: "BC16 CS",
        game_size: NonZero::new(0x324000000).unwrap(),
        fmem_max: NonZero::new(0x4000000).unwrap(),
        mini_size: 0x58000000,
        mini_shared: false,
        vsh_size: NonZero::new(0x28000000).unwrap(),
        unk1: 0,
    }),
    None,
    None,
    Some(DmemConfig {
        name: "GL8 normal",
        game_size: NonZero::new(0x170000000).unwrap(),
        fmem_max: NonZero::new(0x40000000).unwrap(),
        mini_size: 0x70000000,
        mini_shared: true,
        vsh_size: NonZero::new(0x27C00000).unwrap(),
        unk1: 0,
    }),
    None,
    None,
    Some(DmemConfig {
        name: "GL8 kratos",
        game_size: NonZero::new(0x170000000).unwrap(),
        fmem_max: NonZero::new(0x40000000).unwrap(),
        mini_size: 0x70000000,
        mini_shared: true,
        vsh_size: NonZero::new(0x27C00000).unwrap(),
        unk1: 0xA00000,
    }),
    Some(DmemConfig {
        name: "GL8 release",
        game_size: NonZero::new(0x170000000).unwrap(),
        fmem_max: NonZero::new(0x40000000).unwrap(),
        mini_size: 0x70000000,
        mini_shared: true,
        vsh_size: NonZero::new(0x28000000).unwrap(),
        unk1: 0xA00000,
    }),
];
