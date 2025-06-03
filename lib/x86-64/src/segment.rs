use bitfield_struct::bitfield;

/// Raw value of a Global Descriptor-Table Register.
///
/// See Global Descriptor-Table Register section on AMD64 Architecture Programmer's Manual Volume 2
/// for details.
#[repr(C, packed)]
pub struct Gdtr {
    pub limit: u16,
    pub addr: *const SegmentDescriptor,
}

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

/// Raw value of a TSS descriptor.
///
/// See TSS Descriptor section on AMD64 Architecture Programmer's Manual Volume 2 for more details.
#[bitfield(u128)]
pub struct TssDescriptor {
    pub limit1: u16,
    #[bits(24)]
    pub base1: u32,
    #[bits(4)]
    pub ty: u8,
    #[bits(access = None)]
    s: bool,
    #[bits(2)]
    pub dpl: Dpl,
    pub p: bool,
    #[bits(4)]
    pub limit2: u8,
    pub avl: bool,
    #[bits(2)]
    __: u8,
    pub g: bool,
    #[bits(40)]
    pub base2: u64,
    __: u32,
}

/// Raw value of Long Mode TSS.
///
/// See 64-Bit Task State Segment section on AMD64 Architecture Programmer's Manual Volume 2 for
/// more details.
#[repr(C, packed)]
#[derive(Default)]
pub struct Tss64 {
    pub reserved1: u32,
    pub rsp0: u64,
    pub rsp1: u64,
    pub rsp2: u64,
    pub reserved2: u64,
    pub ist1: u64,
    pub ist2: u64,
    pub ist3: u64,
    pub ist4: u64,
    pub ist5: u64,
    pub ist6: u64,
    pub ist7: u64,
    pub reserved3: u64,
    pub reserved4: u16,
    pub io_map_base_address: u16,
}
