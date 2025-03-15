pub use self::vm::*;

mod vm;

/// Implementation of `bios_smap` structure.
///
/// This basically a struct that returned from [e820](https://en.wikipedia.org/wiki/E820). All
/// non-BIOS platform (e.g. UEFI) need to populate this struct from the other sources.
#[repr(C)]
pub struct PhysMap {
    pub base: u64,
    pub len: u64,
    pub ty: MapType,
    pub attrs: u32,
}

/// Type of [PhysMap].
#[repr(u32)]
pub enum MapType {
    None = 0,
    Ram = 1,
    Reserved = 2,
}
