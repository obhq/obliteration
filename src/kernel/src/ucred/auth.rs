/// An implementation of `self_auth_info`.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct AuthInfo {
    pub paid: u64,
    pub caps: [u64; 4],
    pub attrs: [u64; 4],
    pub unk: [u8; 0x40],
}

impl AuthInfo {
    pub const EXE: Self = Self {
        paid: 0x3100000000000001,
        caps: [
            0x2000038000000000,
            0x000000000000FF00,
            0x0000000000000000,
            0x0000000000000000,
        ],
        attrs: [
            0x4000400040000000,
            0x4000000000000000,
            0x0080000000000002,
            0xF0000000FFFF4000,
        ],
        unk: [0; 0x40],
    };
}
