use crate::fs::VPathBuf;
use std::num::TryFromIntError;

/// Input of the syscall entry point.
#[repr(C)]
pub struct Input<'a> {
    pub id: u32,
    pub offset: usize,
    pub module: &'a VPathBuf,
    pub args: [Arg; 6],
}

/// An argument of the syscall.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Arg(usize);

impl<T> From<Arg> for *const T {
    fn from(v: Arg) -> Self {
        v.0 as _
    }
}

impl<T> From<Arg> for *mut T {
    fn from(v: Arg) -> Self {
        v.0 as _
    }
}

impl From<Arg> for usize {
    fn from(v: Arg) -> Self {
        v.0
    }
}

impl TryFrom<Arg> for i32 {
    type Error = TryFromIntError;

    fn try_from(v: Arg) -> Result<Self, Self::Error> {
        TryInto::<u32>::try_into(v.0).map(|v| v as i32)
    }
}

impl TryFrom<Arg> for u32 {
    type Error = TryFromIntError;

    fn try_from(v: Arg) -> Result<Self, Self::Error> {
        v.0.try_into()
    }
}

impl TryFrom<Arg> for crate::memory::Protections {
    type Error = TryFromIntError;

    fn try_from(v: Arg) -> Result<Self, Self::Error> {
        Ok(Self::from_bits_retain(v.0.try_into()?))
    }
}

impl TryFrom<Arg> for crate::memory::MappingFlags {
    type Error = TryFromIntError;

    fn try_from(v: Arg) -> Result<Self, Self::Error> {
        Ok(Self::from_bits_retain(v.0.try_into()?))
    }
}

/// Contains information about the loaded SELF.
#[repr(C)]
pub struct DynlibInfoEx {
    pub size: u64,
    pub name: [u8; 256],
    pub handle: u32,
    pub tlsindex: u32,
    pub tlsinit: usize,
    pub tlsinitsize: u32,
    pub tlssize: u32,
    pub tlsoffset: u32,
    pub tlsalign: u32,
    pub init: usize,
    pub fini: usize,
    pub unk1: u64, // Always zero.
    pub unk2: u64, // Same here.
    pub eh_frame_hdr: usize,
    pub eh_frame: usize,
    pub eh_frame_hdr_size: u32,
    pub eh_frame_size: u32,
    pub mapbase: usize,
    pub textsize: u32,
    pub unk3: u32, // Always 5.
    pub database: usize,
    pub datasize: u32,
    pub unk4: u32,        // Always 3.
    pub unk5: [u8; 0x20], // Always zeroes.
    pub unk6: u32,        // Always 2.
    pub refcount: u32,
}
