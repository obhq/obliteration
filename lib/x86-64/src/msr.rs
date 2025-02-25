use crate::SegmentSelector;
use bitfield_struct::bitfield;

/// Raw value of `EFER` register.
///
/// See Extended Feature Enable Register (EFER) section on AMD64 Architecture Programmer's Manual
/// Volume 2 for more details.
#[bitfield(u64)]
pub struct Efer {
    pub sce: bool,
    #[bits(7)]
    __: u8,
    pub lme: bool,
    __: bool,
    pub lma: bool,
    pub nxe: bool,
    pub svme: bool,
    pub lmsle: bool,
    pub ffxsr: bool,
    pub tce: bool,
    __: bool,
    pub mcommit: bool,
    pub intwb: bool,
    __: bool,
    pub uaie: bool,
    pub aibrse: bool,
    #[bits(42)]
    __: u64,
}

/// Raw value of `STAR` register.
///
/// See SYSCALL and SYSRET section on AMD64 Architecture Programmer's Manual Volume 2 for more
/// details.
#[bitfield(u64)]
pub struct Star {
    pub syscall_eip: u32,
    #[bits(16)]
    pub syscall_sel: SegmentSelector,
    #[bits(16)]
    pub sysret_sel: SegmentSelector,
}
