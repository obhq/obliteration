use serde::{Deserialize, Deserializer};
use std::str::FromStr;
use thiserror::Error;

/// Implementation of [IDPS].
///
/// All fields here are big-endian the same as PS3.
///
/// [IDPS]: https://www.psdevwiki.com/ps3/IDPS
#[repr(C)]
#[derive(Clone)]
pub struct ConsoleId {
    magic: u16,
    company: CompanyId,
    product: ProductId,
    prodsub: u16,
    serial: [u8; 8],
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

impl FromStr for ConsoleId {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

impl<'de> Deserialize<'de> for ConsoleId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        todo!()
    }
}

/// Company identifier for [`ConsoleId`].
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct CompanyId(u16);

impl CompanyId {
    pub const SONY: Self = Self(0x100);
}

/// Product identifier for [`ConsoleId`].
///
/// See https://www.psdevwiki.com/ps4/Console_ID for a list of known IDs.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct ProductId(u16);

#[allow(dead_code)]
impl ProductId {
    pub const DEVKIT: Self = Self(0x8101);
    pub const TESTKIT: Self = Self(0x8201);
    pub const USA: Self = Self(0x8401);
}

/// Represents an error when [`ConsoleId`] fails to construct from a string.
#[derive(Debug, Error)]
pub enum FromStrError {}
