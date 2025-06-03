use crate::MemoryInfo;
use crate::config::Dipsw;
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
        if Self::reserve_phys(mi, 0x1000000.try_into().unwrap(), 1) == 0 {
            panic!("no available memory for high-address memory");
        }

        // TODO: Invoke bootparam_get_ddr3_capacity.
        let mode = Self::load_mode(mi);

        if (0x80C6C0C4u64 & (1 << mode)) != 0 {
            panic!("game DMEM size is not configured");
        }

        // Allocate game DMEM.
        let config = DMEM_CONFIGS[mode].as_ref().unwrap();
        let game = Self::reserve_phys(mi, config.game_size, 0x8000000);

        if game == 0 {
            panic!("not enough memory for game DMEM");
        }

        todo!()
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
}

// TODO: It is likely to be more than 21 entries on PS4 11.00.
static DMEM_CONFIGS: [Option<DmemConfig>; 21] = [
    Some(DmemConfig {
        name: "BC8 normal",
        game_size: NonZero::new(0x148000000).unwrap(),
    }),
    Some(DmemConfig {
        name: "BC8 large",
        game_size: NonZero::new(0x170000000).unwrap(),
    }),
    None,
    Some(DmemConfig {
        name: "BC8 kratos",
        game_size: NonZero::new(0x148000000).unwrap(),
    }),
    Some(DmemConfig {
        name: "BC8 release",
        game_size: NonZero::new(0x148000000).unwrap(),
    }),
    Some(DmemConfig {
        name: "BC8 CS",
        game_size: NonZero::new(0x124000000).unwrap(),
    }),
    None,
    None,
    Some(DmemConfig {
        name: "BC16 normal",
        game_size: NonZero::new(0x148000000).unwrap(),
    }),
    Some(DmemConfig {
        name: "BC16 large",
        game_size: NonZero::new(0x28C000000).unwrap(),
    }),
    Some(DmemConfig {
        name: "BC16 mini-app large",
        game_size: NonZero::new(0x148000000).unwrap(),
    }),
    Some(DmemConfig {
        name: "BC16 kratos",
        game_size: NonZero::new(0x148000000).unwrap(),
    }),
    Some(DmemConfig {
        name: "BC16 release",
        game_size: NonZero::new(0x148000000).unwrap(),
    }),
    Some(DmemConfig {
        name: "BC16 CS",
        game_size: NonZero::new(0x324000000).unwrap(),
    }),
    None,
    None,
    Some(DmemConfig {
        name: "GL8 normal",
        game_size: NonZero::new(0x170000000).unwrap(),
    }),
    None,
    None,
    Some(DmemConfig {
        name: "GL8 kratos",
        game_size: NonZero::new(0x170000000).unwrap(),
    }),
    Some(DmemConfig {
        name: "GL8 release",
        game_size: NonZero::new(0x170000000).unwrap(),
    }),
];
