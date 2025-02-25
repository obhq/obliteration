#![no_std]
pub use self::msr::*;
pub use self::segment::*;

use bitfield_struct::bitfield;

mod msr;
mod segment;

/// Represents a value of `RFLAGS`.
///
/// See RFLAGS Register section on AMD64 Architecture Programmer's Manual Volume 2 for more details.
#[bitfield(u64)]
pub struct Rflags {
    pub cf: bool,
    pub reserved: bool,
    pub pf: bool,
    __: bool,
    pub af: bool,
    __: bool,
    pub zf: bool,
    pub sf: bool,
    pub tf: bool,
    pub r#if: bool,
    pub df: bool,
    pub of: bool,
    #[bits(2)]
    pub iopl: u8,
    pub nt: bool,
    __: bool,
    pub rf: bool,
    pub vm: bool,
    pub ac: bool,
    pub vif: bool,
    pub vip: bool,
    pub id: bool,
    #[bits(42)]
    __: u64,
}
