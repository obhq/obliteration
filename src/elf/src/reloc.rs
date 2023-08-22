use byteorder::{ByteOrder, LE};

/// An iterator over the `Elf64_Rela`.
pub struct Relocations<'a> {
    next: &'a [u8],
}

impl<'a> Relocations<'a> {
    pub(crate) fn new(next: &'a [u8]) -> Self {
        Self { next }
    }
}

impl<'a> Iterator for Relocations<'a> {
    type Item = Relocation;

    fn next(&mut self) -> Option<Self::Item> {
        // Check if all entries has been read.
        if self.next.is_empty() {
            return None;
        }

        // Read the entry.
        let offset = LE::read_u64(self.next);
        let info = LE::read_u64(&self.next[0x08..]);
        let addend = LE::read_i64(&self.next[0x10..]);

        // Move to next entry.
        self.next = &self.next[24..];

        Some(Relocation {
            offset: offset.try_into().unwrap(),
            info,
            addend: addend.try_into().unwrap(),
        })
    }
}

/// An implementation of `Elf64_Rela`.
pub struct Relocation {
    offset: usize,
    info: u64,
    addend: isize,
}

impl Relocation {
    pub const R_X86_64_NONE: u32 = 0;
    pub const R_X86_64_64: u32 = 1;
    pub const R_X86_64_GLOB_DAT: u32 = 6;
    pub const R_X86_64_JUMP_SLOT: u32 = 7;
    pub const R_X86_64_RELATIVE: u32 = 8;
    pub const R_X86_64_DTPMOD64: u32 = 16;

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn ty(&self) -> u32 {
        (self.info & 0xffffffff).try_into().unwrap()
    }

    pub fn symbol(&self) -> usize {
        (self.info >> 32).try_into().unwrap()
    }

    pub fn addend(&self) -> isize {
        self.addend
    }
}
