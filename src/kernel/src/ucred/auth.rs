/// An implementation of `self_auth_info`.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct AuthInfo {
    pub paid: AuthPaid,
    pub caps: AuthCaps,
    pub attrs: AuthAttrs,
    pub unk: [u8; 0x40],
}

impl AuthInfo {
    pub const SYS_CORE: Self = Self {
        paid: AuthPaid::SYS_CORE,
        caps: AuthCaps([
            0x40001C0000000000,
            0x800000000000FF00,
            0x0000000000000000,
            0x0000000000000000,
        ]),
        attrs: AuthAttrs([
            0x4000400080000000,
            0x8000000000000000,
            0x0800000000000000,
            0xF0000000FFFF4000,
        ]),
        unk: [0; 0x40],
    };

    pub fn from_title_id<T: AsRef<str>>(title_id: T) -> Option<Self> {
        // Skip CUSA.
        let id = title_id.as_ref().get(4..)?;

        // Skip leading zeroes.
        let i = id.find(|c| c != '0')?;
        let id: u16 = id[i..].parse().ok()?;

        Some(Self {
            paid: AuthPaid((0x34000003ACC2 << 16) | Into::<u64>::into(id)),
            caps: AuthCaps([
                0x2000038000000000,
                0x000000000000FF00,
                0x0000000000000000,
                0x0000000000000000,
            ]),
            attrs: AuthAttrs([
                0x4000400040000000,
                0x4000000000000000,
                0x0080000000000002,
                0xF0000000FFFF4000,
            ]),
            unk: [0; 0x40],
        })
    }
}

/// A wrapper type for `paid` field of [`AuthInfo`].
///
/// PAID is an abbreviation of "Program Authority ID", not the game has been paid!
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct AuthPaid(u64);

impl AuthPaid {
    pub const KERNEL: Self = Self(0);
    pub const SYS_CORE: Self = Self(0x3800000000000007);

    pub fn get(self) -> u64 {
        self.0
    }
}

/// A wrapper type for `caps` field of [`AuthInfo`].
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct AuthCaps([u64; 4]);

impl AuthCaps {
    pub fn new(raw: [u64; 4]) -> Self {
        Self(raw)
    }

    pub fn is_nongame(&self) -> bool {
        (self.0[0] & 0x1000000000000000) != 0
    }

    pub fn is_user(&self) -> bool {
        (self.0[0] & 0x2000000000000000) != 0
    }

    pub fn is_jit_compiler_process(&self) -> bool {
        todo!()
    }

    pub fn is_jit_application_process(&self) -> bool {
        todo!()
    }

    pub fn has_use_video_service_capability(&self) -> bool {
        self.0[0] >> 0x39 & 1 != 0
    }

    pub fn is_system(&self) -> bool {
        (self.0[0] & 0x4000000000000000) != 0
    }

    pub fn is_unk1(&self) -> bool {
        (self.0[1] & 0x4000000000000000) != 0
    }

    pub fn clear_non_type(&mut self) {
        self.0[0] &= 0x7000000000000000;
        self.0[1] = 0;
        self.0[2] = 0;
        self.0[3] = 0;
    }
}

/// A wrapper type for `caps` field of [`AuthAttrs`].
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct AuthAttrs([u64; 4]);

impl AuthAttrs {
    pub fn new(raw: [u64; 4]) -> Self {
        Self(raw)
    }

    pub fn has_sce_program_attribute(&self) -> bool {
        todo!()
    }

    pub fn is_debuggable_process(&self) -> bool {
        todo!()
    }

    pub fn is_unk1(&self) -> bool {
        (self.0[0] & 0x800000) != 0
    }

    pub fn is_unk2(&self) -> bool {
        (self.0[0] & 0x400000) != 0
    }
}
