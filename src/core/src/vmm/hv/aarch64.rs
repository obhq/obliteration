// SPDX-License-Identifier: MIT OR Apache-2.0
use bitfield_struct::bitfield;

/// Features available on a PE.
pub struct CpuFeats {
    /// Raw value of `ID_AA64MMFR0_EL1`.
    pub mmfr0: Mmfr0,
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
