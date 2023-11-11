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
    // Basic Reloc Types
    pub const R_X86_64_NONE: u32 = 0;
    pub const R_X86_64_64: u32 = 1;
    pub const R_X86_64_PC32: u32 = 2;
    pub const R_X86_64_GOT32: u32 = 3;
    pub const R_X86_64_PLT32: u32 = 4;
    pub const R_X86_64_COPY: u32 = 5;
    pub const R_X86_64_GLOB_DAT: u32 = 6;
    pub const R_X86_64_JUMP_SLOT: u32 = 7;
    pub const R_X86_64_RELATIVE: u32 = 8;
    pub const R_X86_64_GOTPCREL: u32 = 9;
    pub const R_X86_64_32: u32 = 10;
    pub const R_X86_64_32S: u32 = 11;
    pub const R_X86_64_16: u32 = 12;
    pub const R_X86_64_PC16: u32 = 13;
    pub const R_X86_64_8: u32 = 14;
    pub const R_X86_64_PC8: u32 = 15;
    pub const R_X86_64_DTPMOD64: u32 = 16;
    pub const R_X86_64_DTPOFF64: u32 = 17;
    pub const R_X86_64_TPOFF64: u32 = 18;
    pub const R_X86_64_TLSGD: u32 = 19;
    pub const R_X86_64_TLSLD: u32 = 20;
    pub const R_X86_64_DTPOFF32: u32 = 21;
    pub const R_X86_64_GOTTPOFF: u32 = 22;
    pub const R_X86_64_TPOFF32: u32 = 23;
    pub const R_X86_64_PC64: u32 = 24;
    pub const R_X86_64_GOTOFF64: u32 = 25;
    pub const R_X86_64_GOTPC32: u32 = 26;
    pub const R_X86_64_GOT64: u32 = 27;
    pub const R_X86_64_GOTPCREL64: u32 = 28;
    pub const R_X86_64_GOTPC64: u32 = 29;
    pub const R_X86_64_GOTPLT64: u32 = 30;
    pub const R_X86_64_PLTOFF64: u32 = 31;
    // PS4 Reloc Types - 1
    pub const R_X86_64_SIZE32: u32 = 32;
    pub const R_X86_64_SIZE64: u32 = 33;
    pub const R_X86_64_GOTPC32_TLSDESC: u32 = 34;
    pub const R_X86_64_TLSDESC_CALL: u32 = 35;
    pub const R_X86_64_TLSDESC: u32 = 36;
    pub const R_X86_64_IRELATIVE: u32 = 37;
    // Name differs between Orbis bin and Orbis clang?
    pub const R_X86_64_ORBIS_GOTPCREL_LOAD: u32 = 40;
    pub const R_X86_64_PS4_GOTPCREL_LOAD: u32 = 40;
    // PS4 Reloc Types - 2
    pub const R_X86_64_GOTPCRELX: u32 = 41;
    pub const R_X86_64_REX_GOTPCRELX: u32 = 42;

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
