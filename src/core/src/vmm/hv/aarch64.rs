// SPDX-License-Identifier: MIT OR Apache-2.0

/// Features available on a PE.
pub struct CpuFeats {
    /// Indicates support for disabling context synchronizing exception entry and exit.
    ///
    /// - `false`: All exception entries and exits are context synchronization events.
    /// - `true`: Non-context synchronizing exception entry and exit are supported.
    pub feat_exs: bool,
    /// Indicates support for 4KB memory translation granule size.
    pub tgran4: bool,
    /// Indicates support for 64KB memory translation granule size.
    pub tgran64: bool,
    /// Indicates support for 16KB memory translation granule size.
    pub tgran16: bool,
    /// Indicates support for mixed-endian configuration.
    ///
    /// - `false`: No mixed-endian support. The `SCTLR_ELx.EE` bits have a fixed value. See the
    ///   [`Self::big_end_el0`], for whether EL0 supports mixed-endian.
    /// - `true`: Mixed-endian support. The `SCTLR_ELx.EE` and `SCTLR_EL1.E0E` bits can be
    ///   configured.
    pub big_end: bool,
    /// Indicates support for mixed-endian at EL0 only.
    ///
    /// - `false`: No mixed-endian support at EL0. The `SCTLR_EL1.E0E` bit has a fixed value.
    /// - `true`: Mixed-endian support at EL0. The `SCTLR_EL1.E0E` bit can be configured.
    pub big_end_el0: Option<bool>,
    /// Indicates support for ASID 16 bits.
    pub asid16: bool,
    /// Physical Address range supported.
    ///
    /// - `0b0000`: 32 bits, 4GB.
    /// - `0b0001`: 36 bits, 64GB.
    /// - `0b0010`: 40 bits, 1TB.
    /// - `0b0011`: 42 bits, 4TB.
    /// - `0b0100`: 44 bits, 16TB.
    /// - `0b0101`: 48 bits, 256TB.
    pub pa_range: u8,
}
