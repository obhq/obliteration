// SPDX-License-Identifier: MIT OR Apache-2.0
use bitfield_struct::bitfield;

/// Features available on a PE.
#[derive(Default, Clone)]
pub struct CpuFeats {
    /// Raw value of `ID_AA64MMFR0_EL1`.
    pub mmfr0: Mmfr0,
    /// Raw value of `ID_AA64MMFR1_EL1`.
    pub mmfr1: Mmfr1,
    /// Raw value of `ID_AA64MMFR2_EL1`.
    pub mmfr2: Mmfr2,
}

/// Represents a value of `PSTATE`.
///
/// This has the same structure as `SPSR_EL1` when exception taken from AArch64 state.
#[bitfield(u64)]
pub struct Pstate {
    #[bits(4)]
    pub m: u8,
    #[bits(2)]
    __: u8,
    pub f: bool,
    pub i: bool,
    pub a: bool,
    pub d: bool,
    #[bits(2)]
    pub btype: u8,
    pub ssbs: bool,
    pub allint: bool,
    #[bits(6)]
    __: u8,
    pub il: bool,
    pub ss: bool,
    pub pan: bool,
    pub uao: bool,
    pub dit: bool,
    pub tco: bool,
    #[bits(2)]
    __: u8,
    pub v: bool,
    pub c: bool,
    pub z: bool,
    pub n: bool,
    pub pm: bool,
    pub ppend: bool,
    pub exlock: bool,
    #[bits(29)]
    __: u32,
}

/// Represents a value of `SCTLR_EL1`.
#[bitfield(u64)]
pub struct Sctlr {
    pub m: bool,
    pub a: bool,
    pub c: bool,
    pub sa: bool,
    pub sa0: bool,
    pub cp15ben: bool,
    pub naa: bool,
    pub itd: bool,
    pub sed: bool,
    pub uma: bool,
    pub enrctx: bool,
    pub eos: bool,
    pub i: bool,
    pub endb: bool,
    pub dze: bool,
    pub uct: bool,
    pub ntwi: bool,
    __: bool,
    pub ntwe: bool,
    pub wxn: bool,
    pub tscxt: bool,
    pub iesb: bool,
    pub eis: bool,
    pub span: bool,
    pub e0e: bool,
    pub ee: bool,
    pub uci: bool,
    pub enda: bool,
    pub ntlsmd: bool,
    pub lsmaoe: bool,
    pub enib: bool,
    pub enia: bool,
    pub cmow: bool,
    pub mscen: bool,
    __: bool,
    pub bt0: bool,
    pub bt1: bool,
    pub itfsb: bool,
    #[bits(2)]
    pub tcf0: u8,
    #[bits(2)]
    pub tcf: u8,
    pub ata0: bool,
    pub ata: bool,
    pub dssbs: bool,
    pub tweden: bool,
    #[bits(4)]
    pub twedel: u8,
    pub tmt0: bool,
    pub tmt: bool,
    pub tme0: bool,
    pub tme: bool,
    pub enasr: bool,
    pub enas0: bool,
    pub enals: bool,
    pub epan: bool,
    pub tcso0: bool,
    pub tcso: bool,
    pub entp2: bool,
    pub nmi: bool,
    pub spintmask: bool,
    pub tidcp: bool,
}

/// Represents a value of `TCR_EL1`.
#[bitfield(u64)]
pub struct Tcr {
    #[bits(6)]
    pub t0sz: u8,
    __: bool,
    pub epd0: bool,
    #[bits(2)]
    pub irgn0: u8,
    #[bits(2)]
    pub orgn0: u8,
    #[bits(2)]
    pub sh0: u8,
    #[bits(2)]
    pub tg0: u8,
    #[bits(6)]
    pub t1sz: u8,
    pub a1: bool,
    pub epd1: bool,
    #[bits(2)]
    pub irgn1: u8,
    #[bits(2)]
    pub orgn1: u8,
    #[bits(2)]
    pub sh1: u8,
    #[bits(2)]
    pub tg1: u8,
    #[bits(3)]
    pub ips: u8,
    __: bool,
    pub asid: bool,
    pub tbi0: bool,
    pub tbi1: bool,
    pub ha: bool,
    pub hd: bool,
    pub hpd0: bool,
    pub hpd1: bool,
    pub hwu059: bool,
    pub hwu060: bool,
    pub hwu061: bool,
    pub hwu062: bool,
    pub hwu159: bool,
    pub hwu160: bool,
    pub hwu161: bool,
    pub hwu162: bool,
    pub tbid0: bool,
    pub tbid1: bool,
    pub nfd0: bool,
    pub nfd1: bool,
    pub e0pd0: bool,
    pub e0pd1: bool,
    pub tcma0: bool,
    pub tcma1: bool,
    pub ds: bool,
    pub mtx0: bool,
    pub mtx1: bool,
    __: bool,
    __: bool,
}

/// Represents a value of `ID_AA64MMFR0_EL1`.
///
/// All documentation copied from Arm Architecture Reference Manual for A-profile architecture.
#[bitfield(u64)]
pub struct Mmfr0 {
    /// Physical Address range supported.
    ///
    /// - `0b0000`: 32 bits, 4GB.
    /// - `0b0001`: 36 bits, 64GB.
    /// - `0b0010`: 40 bits, 1TB.
    /// - `0b0011`: 42 bits, 4TB.
    /// - `0b0100`: 44 bits, 16TB.
    /// - `0b0101`: 48 bits, 256TB.
    /// - `0b0110`: *When FEAT_LPA is implemented or FEAT_LPA2 is implemented:* 52 bits, 4PB.
    /// - `0b0111`: *When FEAT_D128 is implemented:* 56 bits, 64PB.
    ///
    /// All other values are reserved.
    #[bits(4)]
    pub pa_range: u8,
    /// Number of ASID bits.
    ///
    /// - `0b0000`: 8 bits.
    /// - `0b0010`: 16 bits.
    ///
    /// All other values are reserved.
    #[bits(4)]
    pub asid_bits: u8,
    /// Indicates support for mixed-endian configuration.
    ///
    /// - `0b0000`: No mixed-endian support. The SCTLR_ELx.EE bits have a fixed value. See the
    ///   `big_end_el0` field, for whether EL0 supports mixed-endian.
    /// - `0b0001`: Mixed-endian support. The SCTLR_ELx.EE and SCTLR_EL1.E0E bits can be configured.
    ///
    /// All other values are reserved.
    #[bits(4)]
    pub big_end: u8,
    /// Indicates support for a distinction between Secure and Non-secure Memory.
    ///
    /// - `0b0000`: Does not support a distinction between Secure and Non-secure Memory.
    /// - `0b0001`: Does support a distinction between Secure and Non-secure Memory.
    ///
    /// All other values are reserved.
    ///
    /// If EL3 is implemented, the value `0b0000` is not permitted.
    #[bits(4)]
    pub sns_mem: u8,
    /// Indicates support for mixed-endian at EL0 only.
    ///
    /// - `0b0000`: No mixed-endian support at EL0. The SCTLR_EL1.E0E bit has a fixed value.
    /// - `0b0001`: Mixed-endian support at EL0. The SCTLR_EL1.E0E bit can be configured.
    ///
    /// All other values are reserved.
    ///
    /// This field is invalid and is `RES0` if `big_end` is not `0b0000`.
    #[bits(4)]
    pub big_end_el0: u8,
    /// Indicates support for 16KB memory translation granule size.
    ///
    /// - `0b0000`: 16KB granule not supported.
    /// - `0b0001`: 16KB granule supported.
    /// - `0b0010`: *When FEAT_LPA2 is implemented:* 16KB granule supports 52-bit input addresses
    ///   and can describe 52-bit output addresses.
    ///
    /// All other values are reserved.
    #[bits(4)]
    pub t_gran16: u8,
    /// Indicates support for 64KB memory translation granule size.
    ///
    /// - `0b0000`: 64KB granule supported.
    /// - `0b1111`: 64KB granule not supported.
    ///
    /// All other values are reserved.
    #[bits(4)]
    pub t_gran64: u8,
    /// Indicates support for 4KB memory translation granule size.
    ///
    /// - `0b0000`: 4KB granule supported.
    /// - `0b0001`: *When FEAT_LPA2 is implemented:* 4KB granule supports 52-bit input addresses and
    ///   can describe 52-bit output addresses.
    /// - `0b1111`: 4KB granule not supported.
    ///
    /// All other values are reserved.
    #[bits(4)]
    pub t_gran4: u8,
    /// Indicates support for 16KB memory granule size at stage 2.
    ///
    /// - `0b0000`: Support for 16KB granule at stage 2 is identified in the `t_gran16` field.
    /// - `0b0001`: 16KB granule not supported at stage 2.
    /// - `0b0010`: 16KB granule supported at stage 2.
    /// - `0b0011`: *When FEAT_LPA2 is implemented:* 16KB granule at stage 2 supports 52-bit input
    ///   addresses and can describe 52-bit output addresses.
    ///
    /// All other values are reserved.
    #[bits(4)]
    pub t_gran16_2: u8,
    /// Indicates support for 64KB memory granule size at stage 2.
    ///
    /// - `0b0000`: Support for 64KB granule at stage 2 is identified in the `t_gran64` field.
    /// - `0b0001`: 64KB granule not supported at stage 2.
    /// - `0b0010`: 64KB granule supported at stage 2.
    ///
    /// All other values are reserved.
    #[bits(4)]
    pub t_gran64_2: u8,
    /// Indicates support for 4KB memory granule size at stage 2.
    ///
    /// - `0b0000`: Support for 4KB granule at stage 2 is identified in the `t_gran4` field.
    /// - `0b0001`: 4KB granule not supported at stage 2.
    /// - `0b0010`: 4KB granule supported at stage 2.
    /// - `0b0011`: *When FEAT_LPA2 is implemented:* 4KB granule at stage 2 supports 52-bit input
    ///   addresses and can describe 52-bit output addresses.
    ///
    /// All other values are reserved.
    #[bits(4)]
    pub t_gran4_2: u8,
    /// Indicates support for disabling context synchronizing exception entry and exit.
    ///
    /// - `0b0000`: All exception entries and exits are context synchronization events.
    /// - `0b0001`: Non-context synchronizing exception entry and exit are supported.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_ExS implements the functionality identified by the value `0b0001`.
    #[bits(4)]
    pub exs: u8,
    __: u8,
    /// Indicates presence of the Fine-Grained Trap controls.
    ///
    /// - `0b0000`: Fine-grained trap controls are not implemented.
    /// - `0b0001`: Fine-grained trap controls are implemented. Supports:
    ///   - If EL2 is implemented, the HAFGRTR_EL2, HDFGRTR_EL2, HDFGWTR_EL2, HFGRTR_EL2, HFGITR_EL2
    ///     and HFGWTR_EL2 registers, and their associated traps.
    ///   - If EL2 is implemented, MDCR_EL2.TDCC.
    ///   - If EL3 is implemented, MDCR_EL3.TDCC.
    ///   - If both EL2 and EL3 are implemented, SCR_EL3.FGTEn.
    /// - `0b0010`: As `0b0001`, and also includes support for:
    ///   - If EL2 is implemented, the HDFGRTR2_EL2, HDFGWTR2_EL2, HFGITR2_EL2, HFGRTR2_EL2, and
    ///     HFGWTR2_EL2 registers, and their associated traps.
    ///   - If both EL2 and EL3 are implemented, SCR_EL3.FGTEn2.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_FGT implements the functionality identified by the value `0b0001`.
    ///
    /// FEAT_FGT2 implements the functionality identified by the value `0b0010`.
    ///
    /// From Armv8.6, the value `0b0000` is not permitted.
    ///
    /// From Armv8.9, the value `0b0001` is not permitted.
    #[bits(4)]
    pub fgt: u8,
    /// Indicates presence of Enhanced Counter Virtualization.
    ///
    /// - `0b0000`: Enhanced Counter Virtualization is not implemented.
    /// - `0b0001`: Enhanced Counter Virtualization is implemented. Supports
    ///   CNTHCTL_EL2.{EL1TVT, EL1TVCT, EL1NVPCT, EL1NVVCT, EVNTIS}, CNTKCTL_EL1.EVNTIS,
    ///   CNTPCTSS_EL0 counter views, and CNTVCTSS_EL0 counter views. Extends the PMSCR_EL1.PCT,
    ///   PMSCR_EL2.PCT, TRFCR_EL1.TS, and TRFCR_EL2.TS fields.
    /// - `0b0010`: As `0b0001`, and also includes support for CNTHCTL_EL2.ECV and CNTPOFF_EL2.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_ECV implements the functionality identified by the values `0b0001` and `0b0010`
    ///
    /// From Armv8.6, the only permitted values are `0b0001` and `0b0010`.
    #[bits(4)]
    pub ecv: u8,
}

/// Represents a value of `ID_AA64MMFR1_EL1`.
///
/// All documentation copied from Arm Architecture Reference Manual for A-profile architecture.
#[bitfield(u64)]
pub struct Mmfr1 {
    /// Hardware updates to Access flag and Dirty state in translation tables.
    ///
    /// - `0b0000`: Hardware update of the Access flag and dirty state are not supported.
    /// - `0b0001`: Support for hardware update of the Access flag for Block and Page descriptors.
    /// - `0b0010`: As `0b0001`, and adds support for hardware update of the Access flag for Block
    ///   and Page descriptors. Hardware update of dirty state is supported.
    /// - `0b0011`: As `0b0010`, and adds support for hardware update of the Access flag for Table
    ///   descriptors.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_HAFDBS implements the functionality identified by the values `0b0001` and `0b0010`.
    ///
    /// FEAT_HAFT implements the functionality identified by the value `0b0011`.
    #[bits(4)]
    pub hafdbs: u8,
    /// Number of VMID bits.
    ///
    /// - `0b0000`: 8 bits
    /// - `0b0010`: 16 bits
    ///
    /// All other values are reserved.
    ///
    /// FEAT_VMID16 implements the functionality identified by the value `0b0010`.
    ///
    /// From Armv8.1, the permitted values are `0b0000` and `0b0010`.
    #[bits(4)]
    pub vmid_bits: u8,
    /// Virtualization Host Extensions.
    ///
    /// - `0b0000`: Virtualization Host Extensions not supported.
    /// - `0b0001`: Virtualization Host Extensions supported.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_VHE implements the functionality identified by the value `0b0001`.
    ///
    /// From Armv8.1, the only permitted value is `0b0001`.
    #[bits(4)]
    pub vh: u8,
    /// Hierarchical Permission Disables. Indicates support for disabling hierarchical controls in
    /// translation tables.
    ///
    /// - `0b0000`: Disabling of hierarchical controls not supported.
    /// - `0b0001`: Disabling of hierarchical controls supported with the TCR_EL1.{HPD1, HPD0},
    ///   TCR_EL2.HPD or TCR_EL2.{HPD1, HPD0}, and TCR_EL3.HPD bits.
    /// - `0b0010`: As for value `0b0001`, and adds possible hardware allocation of bits[62:59] of
    ///   the Translation table descriptors from the final lookup level for `IMPLEMENTATION DEFINED`
    ///   use.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_HPDS implements the functionality identified by the value `0b0001`.
    ///
    /// FEAT_HPDS2 implements the functionality identified by the value `0b0010`.
    ///
    /// From Armv8.1, the value `0b0000` is not permitted.
    #[bits(4)]
    pub hpds: u8,
    /// LORegions. Indicates support for LORegions.
    ///
    /// - `0b0000`: LORegions not supported.
    /// - `0b0001`: LORegions supported.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_LOR implements the functionality identified by the value `0b0001`.
    ///
    /// From Armv8.1, the only permitted value is `0b0001`.
    #[bits(4)]
    pub lo: u8,
    /// Privileged Access Never. Indicates support for the PAN bit in PSTATE, SPSR_EL1, SPSR_EL2,
    /// SPSR_EL3, and DSPSR_EL0.
    ///
    /// - `0b0000`: PAN not supported.
    /// - `0b0001`: PAN supported.
    /// - `0b0010`: PAN supported and AT S1E1RP and AT S1E1WP instructions supported.
    /// - `0b0011`: PAN supported, AT S1E1RP and AT S1E1WP instructions supported, and
    ///   SCTLR_EL1.EPAN and SCTLR_EL2.EPAN bits supported.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_PAN implements the functionality identified by the value `0b0001`.
    ///
    /// FEAT_PAN2 implements the functionality added by the value `0b0010`.
    ///
    /// FEAT_PAN3 implements the functionality added by the value `0b0011`.
    ///
    /// In Armv8.1, the permitted values are `0b0001`, `0b0010`, and `0b0011`.
    ///
    /// From Armv8.2, the permitted values are `0b0010` and `0b0011`.
    ///
    /// From Armv8.7, the only permitted value is `0b0011`.
    #[bits(4)]
    pub pan: u8,
    /// ***When FEAT_RAS is implemented:***
    ///
    /// Describes whether the PE can generate SError exceptions from speculative reads of memory,
    /// including speculative instruction fetches.
    ///
    /// - `0b0000`: The PE never generates an SError exception due to an External abort on a
    ///   speculative read.
    /// - `0b0001`: The PE might generate an SError exception due to an External abort on a
    ///   speculative read.
    ///
    /// All other values are reserved.
    ///
    /// ***Otherwise:***
    ///
    /// Reserved, `RES0`.
    #[bits(4)]
    pub spec_sei: u8,
    /// Indicates support for execute-never control distinction by Exception level at stage 2.
    ///
    /// - `0b0000`: Distinction between EL0 and EL1 execute-never control at stage 2 not supported.
    /// - `0b0001`: Distinction between EL0 and EL1 execute-never control at stage 2 supported.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_XNX implements the functionality identified by the value `0b0001`.
    ///
    /// From Armv8.2, the only permitted value is `0b0001`.
    #[bits(4)]
    pub xnx: u8,
    /// Indicates support for the configurable delayed trapping of WFE.
    ///
    /// - `0b0000`: Configurable delayed trapping of WFE is not supported.
    /// - `0b0001`: Configurable delayed trapping of WFE is supported.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_TWED implements the functionality identified by the value `0b0001`.
    ///
    /// From Armv8.6, the permitted values are `0b0000` and `0b0001`.
    #[bits(4)]
    pub twed: u8,
    /// Indicates support for Enhanced Translation Synchronization.
    ///
    /// - `0b0000`: Enhanced Translation Synchronization is not supported.
    /// - `0b0001`: Enhanced Translation Synchronization is not supported.
    /// - `0b0010`: Enhanced Translation Synchronization is supported.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_ETS2 implements the functionality identified by the value `0b0010`.
    ///
    /// From Armv8.8, the values `0b0000` and `0b0001` are not permitted.
    #[bits(4)]
    pub ets: u8,
    /// Indicates support for HCRX_EL2 and its associated EL3 trap.
    ///
    /// - `0b0000`: HCRX_EL2 and its associated EL3 trap are not supported.
    /// - `0b0001`: HCRX_EL2 and its associated EL3 trap are supported.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_HCX implements the functionality identified by the value `0b0001`.
    ///
    /// From Armv8.7, if EL2 is implemented, the only permitted value is `0b0001`.
    #[bits(4)]
    pub hcx: u8,
    /// Indicates support for FPCR.{AH, FIZ, NEP}.
    ///
    /// - `0b0000`: The FPCR.{AH, FIZ, NEP} fields are not supported.
    /// - `0b0001`: The FPCR.{AH, FIZ, NEP} fields are supported.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_AFP implements the functionality identified by the value `0b0001`.
    ///
    /// From Armv8.7, if Advanced SIMD and floating-point is implemented, the only permitted value
    /// is `0b0001`.
    #[bits(4)]
    pub afp: u8,
    /// Indicates support for intermediate caching of translation table walks.
    ///
    /// - `0b0000`: The intermediate caching of translation table walks might include non-coherent
    ///   physical translation caches.
    /// - `0b0001`: The intermediate caching of translation table walks does not include
    ///   non-coherent physical translation caches.
    ///
    /// All other values are reserved.
    ///
    /// Non-coherent physical translation caches are non-coherent caches of previous valid
    /// translation table entries since the last completed relevant TLBI applicable to the PE, where
    /// either:
    ///
    /// - The caching is indexed by the physical address of the location holding the translation
    ///   table entry.
    /// - The caching is used for stage 1 translations and is indexed by the intermediate physical
    ///   address of the location holding the translation table entry.
    ///
    /// FEAT_nTLBPA implements the functionality identified by the value `0b0001`.
    ///
    /// From Armv8.0, the permitted values are `0b0000` and `0b0001`.
    #[bits(4)]
    pub n_tlbpa: u8,
    /// Indicates whether SCTLR_EL1.TIDCP and SCTLR_EL2.TIDCP are implemented in AArch64 state.
    ///
    /// - `0b0000`: SCTLR_EL1.TIDCP and SCTLR_EL2.TIDCP bits are not implemented and are `RES0`.
    /// - `0b0001`: SCTLR_EL1.TIDCP bit is implemented. If EL2 is implemented, SCTLR_EL2.TIDCP bit
    ///   is implemented.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_TIDCP1 implements the functionality identified by the value `0b0001`.
    ///
    /// From Armv8.8, the only permitted value is `0b0001`.
    #[bits(4)]
    pub tidcp1: u8,
    /// Indicates support for cache maintenance instruction permission.
    ///
    /// - `0b0000`: SCTLR_EL1.CMOW, SCTLR_EL2.CMOW, and HCRX_EL2.CMOW bits are not implemented.
    /// - `0b0001`: SCTLR_EL1.CMOW is implemented. If EL2 is implemented, SCTLR_EL2.CMOW and
    ///   HCRX_EL2.CMOW bits are implemented.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_CMOW implements the functionality identified by the value `0b0001`.
    ///
    /// From Armv8.8, the only permitted value is `0b0001`.
    #[bits(4)]
    pub cmow: u8,
    /// Indicates support for restrictions on branch history speculation around exceptions.
    ///
    /// - `0b0000`: The implementation does not disclose whether the branch history information
    ///   created in a context before an exception to a higher Exception level using AArch64 can be
    ///   used by code before that exception to exploitatively control the execution of any indirect
    ///   branches in code in a different context after the exception.
    /// - `0b0001`: The branch history information created in a context before an exception to a
    ///   higher Exception level using AArch64 cannot be used by code before that exception to
    ///   exploitatively control the execution of any indirect branches in code in a different
    ///   context after the exception.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_ECBHB implements the functionality identified by the value `0b0001`.
    ///
    /// From Armv8.9, the value `0b0000` is not permitted.
    #[bits(4)]
    pub ecbhb: u8,
}

/// Represents a value of `ID_AA64MMFR2_EL1`.
///
/// All documentation copied from Arm Architecture Reference Manual for A-profile architecture.
#[bitfield(u64)]
pub struct Mmfr2 {
    /// Indicates support for Common not Private translations.
    ///
    /// - `0b0000`: Common not Private translations not supported.
    /// - `0b0001`: Common not Private translations supported.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_TTCNP implements the functionality identified by the value `0b0001`.
    ///
    /// From Armv8.2, the only permitted value is `0b0001`.
    #[bits(4)]
    pub cnp: u8,
    /// User Access Override.
    ///
    /// - `0b0000`: UAO not supported.
    /// - `0b0001`: UAO supported.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_UAO implements the functionality identified by the value `0b0001`.
    ///
    /// From Armv8.2, the only permitted value is `0b0001`.
    #[bits(4)]
    pub uao: u8,
    /// Indicates support for LSMAOE and nTLSMD bits in SCTLR_EL1 and SCTLR_EL2.
    ///
    /// - `0b0000`: LSMAOE and nTLSMD bits not supported.
    /// - `0b0001`: LSMAOE and nTLSMD bits supported.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_LSMAOC implements the functionality identified by the value `0b0001`.
    #[bits(4)]
    pub lsm: u8,
    /// Indicates support for the IESB bit in the SCTLR_ELx registers.
    ///
    /// - `0b0000`: IESB bit in the SCTLR_ELx registers is not supported.
    /// - `0b0001`: IESB bit in the SCTLR_ELx registers is supported.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_IESB implements the functionality identified by the value `0b0001`.
    #[bits(4)]
    pub iesb: u8,
    /// Indicates support for a larger virtual address.
    ///
    /// - `0b0000`: VMSAv8-64 supports 48-bit VAs.
    /// - `0b0001`: VMSAv8-64 supports 52-bit VAs when using the 64KB translation granule. The size
    ///   for other translation granules is not defined by this field.
    /// - `0b0010`: *When FEAT_D128 is implemented:* VMSAv9-128 supports 56-bit VAs.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_LVA implements the functionality identified by the value `0b0001`.
    ///
    /// FEAT_LVA3 implements the functionality identified by the value `0b0010`.
    #[bits(4)]
    pub va_range: u8,
    /// Support for the use of revised CCSIDR_EL1 register format.
    ///
    /// - `0b0000`: 32-bit format implemented for all levels of the CCSIDR_EL1.
    /// - `0b0001`: 64-bit format implemented for all levels of the CCSIDR_EL1.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_CCIDX implements the functionality identified by the value `0b0001`.
    ///
    /// From Armv8.3, the permitted values are `0b0000` and `0b0001`.
    #[bits(4)]
    pub ccidx: u8,
    /// Nested Virtualization. If EL2 is implemented, indicates support for the use of nested
    /// virtualization.
    ///
    /// - `0b0000`: Nested virtualization is not supported.
    /// - `0b0001`: The HCR_EL2.{AT, NV1, NV} bits are implemented.
    /// - `0b0010`: The VNCR_EL2 register and the HCR_EL2.{NV2, AT, NV1, NV} bits are implemented.
    ///
    /// All other values are reserved.
    ///
    /// If EL2 is not implemented, the only permitted value is `0b0000`.
    ///
    /// FEAT_NV implements the functionality identified by the value `0b0001`.
    ///
    /// FEAT_NV2 implements the functionality identified by the value `0b0010`.
    ///
    /// In Armv8.3, if EL2 is implemented, the permitted values are `0b0000` and `0b0001`.
    ///
    /// From Armv8.4, if EL2 is implemented, the permitted values are `0b0000`, `0b0001`, and
    /// `0b0010`.
    #[bits(4)]
    pub nv: u8,
    /// Identifies support for small translation tables.
    ///
    /// - `0b0000`: The maximum value of the TCR_ELx.{T0SZ,T1SZ} and VTCR_EL2.T0SZ fields is 39.
    /// - `0b0001`: The maximum value of the TCR_ELx.{T0SZ,T1SZ} and VTCR_EL2.T0SZ fields is 48 for
    ///   4KB and 16KB granules, and 47 for 64KB granules.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_TTST implements the functionality identified by the value `0b0001`.
    ///
    /// When FEAT_SEL2 is implemented, the value `0b0000` is not permitted.
    #[bits(4)]
    pub st: u8,
    /// Identifies support for unaligned single-copy atomicity and atomic functions.
    ///
    /// - `0b0000`: Unaligned single-copy atomicity and atomic functions are not supported.
    /// - `0b0001`: Unaligned single-copy atomicity and atomic functions with a 16-byte address
    ///   range aligned to 16-bytes are supported.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_LSE2 implements the functionality identified by the value `0b0001`.
    ///
    /// In Armv8.2, the permitted values are `0b0000` and `0b0001`.
    ///
    /// From Armv8.4, the only permitted value is `0b0001`.
    #[bits(4)]
    pub at: u8,
    /// Indicates the value of ESR_ELx.EC that reports an exception generated by a read access to
    /// the feature ID space.
    ///
    /// - `0b0000`: An exception which is generated by a read access to the feature ID space, other
    ///   than a trap caused by HCR_EL2.TIDx, SCTLR_EL1.UCT, or SCTLR_EL2.UCT, is reported by
    ///   ESR_ELx.EC == `0x0`.
    /// - `0b0001`: All exceptions generated by an AArch64 read access to the feature ID space are
    ///   reported by ESR_ELx.EC == `0x18`.
    ///
    /// All other values are reserved.
    ///
    /// The Feature ID space is defined as the System register space in AArch64 with op0==3,
    /// op1=={0, 1, 3}, CRn==0, CRm=={0-7}, op2=={0-7}.
    ///
    /// FEAT_IDST implements the functionality identified by the value `0b0001`.
    ///
    /// From Armv8.4, the only permitted value is `0b0001`.
    #[bits(4)]
    pub ids: u8,
    /// Indicates support for HCR_EL2.FWB.
    ///
    /// - `0b0000`: HCR_EL2.FWB bit is not supported.
    /// - `0b0001`: HCR_EL2.FWB is supported.
    ///
    /// All other values reserved.
    ///
    /// FEAT_S2FWB implements the functionality identified by the value `0b0001`.
    ///
    /// From Armv8.4, the only permitted value is `0b0001`.
    #[bits(4)]
    pub fwb: u8,
    #[bits(4)]
    __: u8,
    /// Indicates support for TTL field in address operations.
    ///
    /// - `0b0000`: TLB maintenance instructions by address have bits[47:44] as `RES0`.
    /// - `0b0001`: TLB maintenance instructions by address have bits[47:44] holding the TTL field.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_TTL implements the functionality identified by the value `0b0001`.
    ///
    /// This field affects TLBI IPAS2E1, TLBI IPAS2E1NXS, TLBI IPAS2E1IS, TLBI IPAS2E1ISNXS,
    /// TLBI IPAS2E1OS, TLBI IPAS2E1OSNXS, TLBI IPAS2LE1, TLBI IPAS2LE1NXS, TLBI IPAS2LE1IS,
    /// TLBI IPAS2LE1ISNXS, TLBI IPAS2LE1OS, TLBI IPAS2LE1OSNXS, TLBI VAAE1, TLBI VAAE1NXS,
    /// TLBI VAAE1IS, TLBI VAAE1ISNXS, TLBI VAAE1OS, TLBI VAAE1OSNXS, TLBI VAALE1, TLBI VAALE1NXS,
    /// TLBI VAALE1IS, TLBI VAALE1ISNXS, TLBI VAALE1OS, TLBI VAALE1OSNXS, TLBI VAE1, TLBI VAE1NXS,
    /// TLBI VAE1IS, TLBI VAE1ISNXS, TLBI VAE1OS, TLBI VAE1OSNXS, TLBI VAE2, TLBI VAE2NXS,
    /// TLBI VAE2IS, TLBI VAE2ISNXS, TLBI VAE2OS, TLBI VAE2OSNXS, TLBI VAE3, TLBI VAE3NXS,
    /// TLBI VAE3IS, TLBI VAE3ISNXS, TLBI VAE3OS, TLBI VAE3OSNXS,TLBI VALE1, TLBI VALE1NXS,
    /// TLBI VALE1IS, TLBI VALE1ISNXS, TLBI VALE1OS, TLBI VALE1OSNXS, TLBI VALE2, TLBI VALE2NXS,
    /// TLBI VALE2IS, TLBI VALE2ISNXS, TLBI VALE2OS, TLBI VALE2OSNXS, TLBI VALE3, TLBI VALE3NXS,
    /// TLBI VALE3IS, TLBI VALE3ISNXS, TLBI VALE3OS, TLBI VALE3OSNXS.
    ///
    /// From Armv8.4, the only permitted value is `0b0001`.
    #[bits(4)]
    pub ttl: u8,
    /// Allows identification of the requirements of the hardware to have break-before-make
    /// sequences when changing block size for a translation.
    ///
    /// - `0b0000`: Level 0 support for changing block size is supported.
    /// - `0b0001`: Level 1 support for changing block size is supported.
    /// - `0b0010`: Level 2 support for changing block size is supported.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_BBM implements the functionality identified by the values `0b0000`, `0b0001`, and
    /// `0b0010`.
    ///
    /// From Armv8.4, the permitted values are `0b0000`, `0b0001`, and `0b0010`.
    #[bits(4)]
    pub bbm: u8,
    /// Enhanced Virtualization Traps. If EL2 is implemented, indicates support for the
    /// HCR_EL2.{TTLBOS, TTLBIS, TOCU, TICAB, TID4} traps.
    ///
    /// - `0b0000`: HCR_EL2.{TTLBOS, TTLBIS, TOCU, TICAB, TID4} traps are not supported.
    /// - `0b0001`: HCR_EL2.{TOCU, TICAB, TID4} traps are supported. HCR_EL2.{TTLBOS, TTLBIS} traps
    ///   are not supported.
    /// - `0b0010`: HCR_EL2.{TTLBOS, TTLBIS, TOCU, TICAB, TID4} traps are supported.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_EVT implements the functionality identified by the values `0b0001` and `0b0010`.
    ///
    /// If EL2 is not implemented, the only permitted value is `0b0000`.
    ///
    /// In Armv8.2, the permitted values are `0b0000`, `0b0001`, and `0b0010`.
    ///
    /// From Armv8.5, the permitted values are:
    ///
    /// - `0b0000` when EL2 is not implemented.
    /// - `0b0010` when EL2 is implemented.
    #[bits(4)]
    pub evt: u8,
    /// Indicates support for the E0PD mechanism.
    ///
    /// - `0b0000`: E0PDx mechanism is not implemented.
    /// - `0b0001`: E0PDx mechanism is implemented.
    ///
    /// All other values are reserved.
    ///
    /// FEAT_E0PD implements the functionality identified by the value `0b0001`.
    ///
    /// In Armv8.4, the permitted values are `0b0000` and `0b0001`.
    ///
    /// From Armv8.5, the only permitted value is `0b0001`.
    ///
    /// If FEAT_E0PD is implemented, FEAT_CSV3 must be implemented.
    #[bits(4)]
    pub e0pd: u8,
}
