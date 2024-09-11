// SPDX-License-Identifier: MIT OR Apache-2.0
use bitfield_struct::bitfield;

/// Features available on a PE.
pub struct CpuFeats {
    /// Raw value of `ID_AA64MMFR0_EL1`.
    pub mmfr0: Mmfr0,
    /// Raw value of `ID_AA64MMFR1_EL1`.
    pub mmfr1: Mmfr1,
}

/// Represents a value of `PSTATE`.
#[bitfield(u32)]
pub struct Pstate {
    #[bits(4)]
    pub m: u8,
    #[bits(2)]
    __: u8,
    pub f: bool,
    pub i: bool,
    pub a: bool,
    pub d: bool,
    #[bits(22)]
    __: u32,
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
