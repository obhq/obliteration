/// An implementation of PS4 registry manager.
pub struct RegMgr {}

impl RegMgr {
    pub fn new() -> Self {
        Self {}
    }

    pub fn decode_key(&self, v1: u64, _: u32, _: u32) -> i32 {
        // TODO: Verify if the algorithm used here is a standard algorithm.
        let a = (v1 & 0xff) as i32;
        let b = ((v1 >> 8) & 0xff) as i32;
        let c = ((v1 >> 16) & 0xff) as i32;
        let d = ((v1 >> 24) & 0xff) as i32;
        let e = ((v1 >> 32) & 0xff) as i32;
        let f = ((v1 >> 40) & 0xff) as i32;
        let g = (a + b + c + d + e * f) as u16;
        let h = (v1 >> 48) as u16;

        if g == h {
            let i = (f ^ 0x6b) as usize;

            if i > 12 {
                todo!("regmgr_call with multiplier ^ 0x6b > 12");
            }

            let x = e ^ SBOX1[i] as i32;
            let sbox = if x == 0x19 {
                &SBOX2
            } else {
                todo!("regmgr_call with x != 0x19");
            };

            // Construct the key.
            let mut key: u32 = (a ^ (sbox[i + 3] as i32)) as u32;

            key |= ((c ^ (sbox[i + 2] as i32)) as u32) << 8;
            key |= ((d ^ (sbox[i + 1] as i32)) as u32) << 16;
            key |= ((b ^ (sbox[i] as i32)) as u32) << 24;

            // Lookup the entry.
            for e in &ENTRIES {
                if e.key == key {
                    todo!("regmgr_call with direct matched entry");
                } else if e.unk1 >= 2 {
                    for v in 1..e.unk1 {
                        if ((v << (e.unk2 & 0x1f)) + e.key) == key {
                            todo!("regmgr_call with indirect matched entry");
                        }
                    }
                }
            }

            #[allow(overflowing_literals)]
            0x800d0203
        } else {
            todo!("regmgr_call with checksum mismatched");
        }
    }
}

pub struct RegEntry {
    pub key: u32,
    pub unk1: u32,
    pub unk2: u8,
}

const SBOX1: [u8; 13] = [
    0x8c, 0x4c, 0xa4, 0xff, 0x7b, 0xf5, 0xee, 0x63, 0x5a, 0x23, 0x70, 0x9a, 0x03,
];

const SBOX2: [u8; 16] = [
    0x14, 0xee, 0xde, 0xe1, 0x80, 0xac, 0xf3, 0x78, 0x47, 0x43, 0xdb, 0x40, 0x93, 0xdd, 0xb1, 0x34,
];

const ENTRIES: [RegEntry; 560] = [
    RegEntry {
        key: 0x1010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x1020000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x1030000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x1040000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x1050000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x1060000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x1070000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x1080000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x1400000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x1800100,
        unk1: 10,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x1800200,
        unk1: 10,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x2010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2020000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2030000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2040000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2050000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2060000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2070000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2080000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x20a0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x20b0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x20c0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x20e0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x20f0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2100000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2110000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2120000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2800200,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2800300,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2800400,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2800500,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2800600,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2800700,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2800800,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2800900,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2800a00,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2800b00,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2800c00,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2800d00,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2804000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2820100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2820200,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2820300,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2820400,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2820500,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2820600,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2820700,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2820800,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2820900,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2820b00,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2820c00,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2820e00,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2780100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2780200,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2780300,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2780400,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x27c0100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2860100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2860300,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2860500,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2880100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2880200,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2900100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2a00100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2b00100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2b00200,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2b80100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2bc0100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2bc0200,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2bc0300,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2bc0400,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2bc0500,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2bc0600,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2bc0700,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2bc0800,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2bc0900,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2bc0a00,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2bc0b00,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2bc4001,
        unk1: 20,
        unk2: 0x8,
    },
    RegEntry {
        key: 0x2bc4002,
        unk1: 20,
        unk2: 0x8,
    },
    RegEntry {
        key: 0x2bc4003,
        unk1: 20,
        unk2: 0x8,
    },
    RegEntry {
        key: 0x2be0100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2be0200,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2c30100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2c30200,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x3800100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x3800200,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x3800300,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x3800400,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x3800500,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x3800600,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x3800700,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x3800800,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x3800900,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x3800a00,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x5010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x5020000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x5030000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x5040000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x5050000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x5060000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x5070000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x5080000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x5090000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x50a0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x5140000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x5150000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x5160000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x5170000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x5180000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x7010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x7020000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x7030000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x7040000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x7050000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x7060000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x7070000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x7800100,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7800200,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7800300,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7800500,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7800600,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7800700,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7800800,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7800900,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7800a00,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7800b00,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7800c00,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7800d00,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7800e00,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7800f00,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7801000,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7801100,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7805c01,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7805c02,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7805c03,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7805c04,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7805c05,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7805c06,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7805c07,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7806401,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7806402,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7806403,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7806c01,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7806c02,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7806c03,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7807801,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7807c01,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7807c02,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7807c03,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808001,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808002,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808003,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808004,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808005,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808006,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808007,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808008,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808009,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780800a,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780800b,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780800c,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780800d,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780800e,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780800f,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808010,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808011,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808012,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808013,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808014,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808015,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808016,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808017,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808018,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808019,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780801a,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780801b,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780801c,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780801d,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780801e,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780801f,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808080,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808081,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808082,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808083,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808084,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7808085,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7809001,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7809002,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7809003,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7809004,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7809005,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x7809006,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780a001,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780a002,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780a003,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780a004,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780a005,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780a006,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780a007,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780a008,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780a009,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780a00a,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780a00b,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780a00c,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780a00d,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780a00e,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780a00f,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b003,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b004,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b007,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b008,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b009,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b00a,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b00b,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b00c,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b00d,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b00e,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b00f,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b010,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b011,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b012,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b013,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b801,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b802,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b803,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b804,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b805,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b806,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b807,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b808,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b809,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b80a,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b80b,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b80c,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b80d,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b80e,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b80f,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b810,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b811,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b812,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b813,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b814,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b815,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b816,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b817,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b818,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b819,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b81a,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780b81b,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780bc01,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780bc02,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780bc03,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780bd01,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780bd02,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780bd03,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c001,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c002,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c003,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c004,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c005,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c006,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c007,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c008,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c009,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c00a,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c00b,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c00c,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c00d,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c00e,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c010,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c011,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c012,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c013,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c014,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c015,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c016,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c017,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c018,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c019,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c01a,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c01b,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c01c,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c01d,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c01e,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c01f,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c020,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c021,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c022,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c023,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c024,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c025,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c026,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c027,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c028,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c029,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c02a,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c02b,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c02c,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c041,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c042,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c043,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c501,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c502,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c504,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c505,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c506,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c601,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c602,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c603,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780c701,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780d001,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780d002,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780d101,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780d102,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780dc01,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780dc02,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780dc03,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780dc04,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780dc05,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780dc06,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780dc07,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780e001,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780e002,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780e003,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780e004,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780e005,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780e101,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780e401,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780e402,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780e403,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780f801,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780f802,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x780f803,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x9010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x9020000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x9030000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x9040000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x9050000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x9060000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x9070000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x9400100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x9400200,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x9400300,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x9400400,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa030000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa040000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa060000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa070000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa080000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa0a0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa0d0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa0f0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa100000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa110000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa120000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa130000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa140000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa150000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa160000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa170000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa180000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa190000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa1a0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa1b0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xa1c0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xb030000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xb040000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xb050000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xb060000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xb070000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xb080000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xb090000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xb400101,
        unk1: 20,
        unk2: 0x8,
    },
    RegEntry {
        key: 0xc010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xc020000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0xc400101,
        unk1: 20,
        unk2: 0x8,
    },
    RegEntry {
        key: 0x12010100,
        unk1: 32,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x14140100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14140200,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14140300,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14140400,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14140500,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14140600,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14140700,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14140800,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14140c00,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14140d00,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14140e00,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14140f00,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14190100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14190600,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14190700,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14190800,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14190900,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14190a00,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e0100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e0200,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e0300,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e0400,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e0500,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e4001,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e6001,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e6002,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e6003,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e6004,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e6005,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e6006,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e6007,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e6008,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e6009,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e600a,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e600b,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e600c,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e8001,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e8002,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x141e8003,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14230100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14230200,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14230300,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14230400,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14230500,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14230600,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14230700,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14230800,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14234001,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14234002,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14234003,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14234004,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14234005,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14234006,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14234007,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14234008,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14280100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14280200,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14280300,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x142d0100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x142d0200,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x142d0300,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x142d0400,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x142e0100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14320101,
        unk1: 10,
        unk2: 0x8,
    },
    RegEntry {
        key: 0x14320102,
        unk1: 10,
        unk2: 0x8,
    },
    RegEntry {
        key: 0x14320103,
        unk1: 10,
        unk2: 0x8,
    },
    RegEntry {
        key: 0x14320104,
        unk1: 10,
        unk2: 0x8,
    },
    RegEntry {
        key: 0x14700000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14710000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14740000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14750000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14760000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14770000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x14780000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x19010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x19600000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x19800000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x19810000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x1e010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x1e020000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x20010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x20020100,
        unk1: 10,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x20034001,
        unk1: 2,
        unk2: 0x8,
    },
    RegEntry {
        key: 0x20400000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x20410000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x23010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x23020000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x23030000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x23040000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x23050000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x23060000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x23070000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x23080000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x23090000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x230a0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x230b0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x230c0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x230d0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x230e0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x230f0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x28010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x28020000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x28030000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x29010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x29020000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x29030000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2a010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x2d010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x320c0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x37040000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x37050000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x37060000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x37090000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x370a0000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x3c020000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x3c030000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x3c040000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x41810000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x41820000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x41010100,
        unk1: 32,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x41010200,
        unk1: 32,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x41010300,
        unk1: 32,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x41010400,
        unk1: 32,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x42800100,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x42800200,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x42800300,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x42800400,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x43800100,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x43800200,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x43800300,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x43800400,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0xc8800100,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0xc8800200,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0xc8800300,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0xc8800400,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x45010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x45020000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x45030000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x45050000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x45060000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x45400000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x55010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x56010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x64800100,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x64800200,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x64800300,
        unk1: 16,
        unk2: 0x10,
    },
    RegEntry {
        key: 0x46010100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x46010200,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x46010300,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x49010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x49020100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x49020200,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x49020300,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x49020400,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x49020500,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x49020600,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x49020700,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x49020800,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x49020900,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x49020a00,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x49020b00,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x49020c00,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x4b010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x4f010000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x4f400101,
        unk1: 10,
        unk2: 0x8,
    },
    RegEntry {
        key: 0x4f400102,
        unk1: 10,
        unk2: 0x8,
    },
    RegEntry {
        key: 0x4f400103,
        unk1: 10,
        unk2: 0x8,
    },
    RegEntry {
        key: 0x4f400104,
        unk1: 10,
        unk2: 0x8,
    },
    RegEntry {
        key: 0x50400100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x70030000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x70040000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x70050000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x73010100,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x73010200,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x73010300,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x73010400,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x73010500,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x73010600,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x73010700,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x73010800,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x76800000,
        unk1: 0,
        unk2: 0x0,
    },
    RegEntry {
        key: 0x7802c9c8,
        unk1: 0,
        unk2: 0x0,
    },
];
