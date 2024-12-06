use num_enum::{IntoPrimitive, TryFromPrimitive};
use redb::{TableDefinition, TypeName};

pub const FS_TYPE: TableDefinition<(), FsType> = TableDefinition::new("fs_type");

/// Filesystem type.
#[repr(u16)]
#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
pub enum FsType {
    ExFat = 1,
}

impl redb::Value for FsType {
    type SelfType<'a> = Self;
    type AsBytes<'a> = [u8; 2];

    fn fixed_width() -> Option<usize> {
        Some(size_of::<Self>())
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        u16::from_le_bytes(data.try_into().unwrap())
            .try_into()
            .unwrap()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a,
        Self: 'b,
    {
        u16::from(*value).to_le_bytes()
    }

    fn type_name() -> TypeName {
        TypeName::new("obliteration::FsType")
    }
}
