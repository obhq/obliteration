use std::fmt::{Display, Formatter};

pub struct Segment {
    flags: SegmentFlags,
    offset: u64,
    compressed_size: u64,
    decompressed_size: u64,
}

impl Segment {
    pub(super) fn new(
        flags: SegmentFlags,
        offset: u64,
        compressed_size: u64,
        decompressed_size: u64,
    ) -> Self {
        Self {
            flags,
            offset,
            compressed_size,
            decompressed_size,
        }
    }

    pub fn flags(&self) -> SegmentFlags {
        self.flags
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn compressed_size(&self) -> u64 {
        self.compressed_size
    }

    pub fn decompressed_size(&self) -> u64 {
        self.decompressed_size
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct SegmentFlags(u64);

impl SegmentFlags {
    pub fn id(self) -> u32 {
        ((self.0 >> 20) & 0xfff) as _
    }

    pub fn is_ordered(self) -> bool {
        (self.0 & 1) != 0
    }

    pub fn is_encrypted(self) -> bool {
        (self.0 & 2) != 0
    }

    pub fn is_signed(self) -> bool {
        (self.0 & 4) != 0
    }

    pub fn is_compressed(self) -> bool {
        (self.0 & 8) != 0
    }

    pub fn is_blocked(self) -> bool {
        (self.0 & 0x800) != 0
    }
}

impl From<u64> for SegmentFlags {
    fn from(v: u64) -> Self {
        Self(v)
    }
}

impl Display for SegmentFlags {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{:#018x} (", self.0)?;
        write!(f, "id = {}, ", self.id())?;
        write!(f, "is_ordered = {}, ", self.is_ordered())?;
        write!(f, "is_encrypted = {}, ", self.is_encrypted())?;
        write!(f, "is_signed = {}, ", self.is_signed())?;
        write!(f, "is_compressed = {}, ", self.is_compressed())?;
        write!(f, "is_blocked = {}", self.is_blocked())?;
        f.write_str(")")?;

        Ok(())
    }
}
