use bitfield_struct::bitfield;

/// Raw value of a Segment Descriptor.
///
/// See Legacy Segment Descriptors section on AMD64 Architecture Programmer's Manual Volume 2 for
/// more details.
#[bitfield(u64)]
pub struct SegmentDescriptor {
    pub limit1: u16,
    #[bits(24)]
    pub base1: u32,
    #[bits(4)]
    pub ty: u8,
    pub s: bool,
    #[bits(2)]
    pub dpl: Dpl,
    pub p: bool,
    #[bits(4)]
    pub limit2: u8,
    pub avl: bool,
    pub l: bool,
    pub db: bool,
    pub g: bool,
    pub base2: u8,
}

/// Raw value of a Segment Selector (e.g. `CS` and `DS` register).
///
/// See Segment Selectors section on AMD64 Architecture Programmer's Manual Volume 2 for more
/// details.
#[bitfield(u16)]
pub struct SegmentSelector {
    #[bits(2)]
    pub rpl: Dpl,
    #[bits(1)]
    pub ti: Ti,
    #[bits(13)]
    pub si: u16,
}

/// Raw value of Descriptor Privilege-Level field.
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Dpl {
    Ring0,
    Ring1,
    Ring2,
    Ring3,
}

impl Dpl {
    /// # Panics
    /// If `v` is greater than 3.
    pub const fn from_bits(v: u8) -> Self {
        match v {
            0 => Self::Ring0,
            1 => Self::Ring1,
            2 => Self::Ring2,
            3 => Self::Ring3,
            _ => panic!("invalid value"),
        }
    }

    pub const fn into_bits(self) -> u8 {
        self as _
    }
}

/// Raw value of Table Indicator field.
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Ti {
    Gdt,
    Ldt,
}

impl Ti {
    /// # Panics
    /// If `v` is greater than 1.
    pub const fn from_bits(v: u8) -> Self {
        match v {
            0 => Self::Gdt,
            1 => Self::Ldt,
            _ => panic!("invalid value"),
        }
    }

    pub const fn into_bits(self) -> u8 {
        self as _
    }
}
