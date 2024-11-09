use bitfield_struct::bitfield;

/// Raw value of `VBAR_EL1`.
#[bitfield(u64)]
pub struct Vbar {
    #[bits(11)]
    __: u16,
    #[bits(53)]
    pub addr: u64,
}

/// Raw value of `ESR_EL1`, `ESR_EL2` and `ESR_EL3`.
#[bitfield(u64)]
pub struct Esr {
    #[bits(25)]
    pub iss: u32,
    pub il: bool,
    #[bits(6)]
    pub ec: u8,
    #[bits(24)]
    pub iss2: u32,
    __: u8,
}
