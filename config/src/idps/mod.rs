use core::fmt::{Display, Formatter};

/// Implementation of [IDPS].
///
/// All fields here are big-endian the same as PS3.
///
/// [IDPS]: https://www.psdevwiki.com/ps3/IDPS
#[repr(C)]
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ConsoleId {
    magic: u16,
    company: CompanyId,
    pub product: ProductId,
    pub prodsub: u16,
    #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
    pub serial: [u8; 8],
}

impl ConsoleId {
    pub fn new(company: CompanyId, product: ProductId, prodsub: u16, serial: [u8; 8]) -> Self {
        Self {
            magic: 0,
            company,
            product,
            prodsub,
            serial,
        }
    }
}

impl Default for ConsoleId {
    fn default() -> Self {
        Self::new(
            CompanyId::SONY,
            ProductId::USA,
            0x1200,
            [0x10, 0, 0, 0, 0, 0, 0, 0],
        )
    }
}

/// Company identifier for [`ConsoleId`].
#[repr(transparent)]
#[derive(Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct CompanyId(u16);

impl CompanyId {
    pub const SONY: Self = Self(0x100);
}

/// Product identifier for [`ConsoleId`].
///
/// See https://www.psdevwiki.com/ps4/Console_ID for a list of known IDs.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ProductId(u16);

impl ProductId {
    pub const DEVKIT: Self = Self(0x8101);
    pub const TESTKIT: Self = Self(0x8201);
    pub const USA: Self = Self(0x8401);
    pub const SOUTH_ASIA: Self = Self(0x8A01);
}

impl Display for ProductId {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let v = match *self {
            Self::DEVKIT => "TOOL/DEVKIT",
            Self::TESTKIT => "DEX/TESTKIT",
            Self::USA => "UC2/USA/CANADA",
            Self::SOUTH_ASIA => "E12/MALAYSIA",
            _ => return write!(f, "{:#X}", self.0),
        };

        f.write_str(v)
    }
}
