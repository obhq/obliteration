use self::program::Program;
use self::section::Section;
use self::segment::Segment;
use crate::fs::file::File;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, SeekFrom};
use std::ops::IndexMut;
use util::mem::{new_buffer, read_array, read_u16_le, read_u32_le, read_u64_le, read_u8, uninit};

pub mod program;
pub mod section;
pub mod segment;

// https://www.psdevwiki.com/ps4/SELF_File_Format
pub struct Executable {
    file_size: u64,
    entry_addr: usize,
    segments: Vec<Segment>,
    programs: Vec<Program>,
    sections: Vec<Section>,
}

impl Executable {
    pub fn load(file: &mut File) -> Result<Self, LoadError> {
        // Read SELF header.
        let mut hdr: [u8; 32] = uninit();

        if let Err(e) = file.read_exact(&mut hdr) {
            return Err(LoadError::ReadSelfHeaderFailed(e));
        }

        let hdr = hdr.as_ptr();

        // Check magic.
        // Kyty also checking if Category = 0x01 & Program Type = 0x01 & Padding = 0x00.
        // Let's check only magic for now until something is broken.
        let magic: [u8; 8] = read_array(hdr, 0x00);
        let unknown = read_u16_le(hdr, 0x1a);

        if magic != [0x4f, 0x15, 0x3d, 0x1d, 0x00, 0x01, 0x01, 0x12] || unknown != 0x22 {
            return Err(LoadError::InvalidSelfMagic);
        }

        // Load SELF fields.
        let file_size = read_u64_le(hdr, 0x10);

        // Load SELF segment headers.
        let segment_count = read_u16_le(hdr, 0x18) as usize;
        let mut segments: Vec<Segment> = Vec::with_capacity(segment_count);

        for i in 0..segment_count {
            // Read header.
            let mut hdr: [u8; 32] = uninit();

            if let Err(e) = file.read_exact(&mut hdr) {
                return Err(LoadError::ReadSelfSegmentHeaderFailed(i, e));
            }

            let hdr = hdr.as_ptr();

            // Load fields.
            let flags = read_u64_le(hdr, 0);
            let offset = read_u64_le(hdr, 8);
            let compressed_size = read_u64_le(hdr, 16);
            let decompressed_size = read_u64_le(hdr, 24);

            segments.push(Segment::new(
                flags.into(),
                offset,
                compressed_size,
                decompressed_size,
            ));
        }

        // Read ELF header.
        let elf_offset = file.stream_position().unwrap();
        let mut hdr: [u8; 64] = uninit();

        if let Err(e) = file.read_exact(&mut hdr) {
            return Err(LoadError::ReadElfHeaderFailed(e));
        }

        let hdr = hdr.as_ptr();

        // Check ELF magic.
        let magic: [u8; 4] = read_array(hdr, 0x00);

        if magic != [0x7f, 0x45, 0x4c, 0x46] {
            return Err(LoadError::InvalidElfMagic);
        }

        // Check ELF type.
        if read_u8(hdr, 0x04) != 2 {
            return Err(LoadError::UnsupportedBitness);
        }

        if read_u8(hdr, 0x05) != 1 {
            return Err(LoadError::UnsupportedEndianness);
        }

        // Load ELF header.
        let e_entry = read_u64_le(hdr, 0x18);
        let e_phoff = read_u64_le(hdr, 0x20);
        let e_shoff = read_u64_le(hdr, 0x28);
        let e_phnum = read_u16_le(hdr, 0x38);
        let e_shnum = read_u16_le(hdr, 0x3c);
        let e_shstrndx = read_u16_le(hdr, 0x3e);

        // Load program headers.
        let mut programs: Vec<Program> = Vec::with_capacity(e_phnum as _);

        file.seek(SeekFrom::Start(elf_offset + e_phoff)).unwrap();

        for i in 0..e_phnum {
            // Read header.
            let mut hdr: [u8; 0x38] = uninit();

            if let Err(e) = file.read_exact(&mut hdr) {
                return Err(LoadError::ReadProgramHeaderFailed(i as _, e));
            }

            let hdr = hdr.as_ptr();

            // Load fields.
            let p_type = read_u32_le(hdr, 0x00);
            let p_flags = read_u32_le(hdr, 0x04);
            let p_offset = read_u64_le(hdr, 0x08);
            let p_vaddr = read_u64_le(hdr, 0x10);
            let p_filesz = read_u64_le(hdr, 0x20);
            let p_memsz = read_u64_le(hdr, 0x28);
            let p_align = read_u64_le(hdr, 0x30);

            programs.push(Program::new(
                p_type.into(),
                p_flags.into(),
                p_offset,
                p_vaddr as _,
                p_filesz,
                p_memsz as _,
                p_align as _,
            ));
        }

        // Load section headers.
        let mut sections: Vec<Section> = Vec::with_capacity(e_shnum as _);

        file.seek(SeekFrom::Start(elf_offset + e_shoff)).unwrap();

        for i in 0..e_shnum {
            // Read header.
            let mut hdr: [u8; 64] = uninit();

            if let Err(e) = file.read_exact(&mut hdr) {
                return Err(LoadError::ReadSectionHeaderFailed(i as _, e));
            }

            let hdr = hdr.as_ptr();

            // Load fields.
            let sh_name = read_u32_le(hdr, 0);
            let sh_offset = read_u64_le(hdr, 24);
            let sh_size = read_u64_le(hdr, 32);

            sections.push(Section::new(sh_name, sh_offset, sh_size));
        }

        // Load section names.
        if e_shstrndx != 0 {
            // Get section.
            let section = match sections.get(e_shstrndx as usize) {
                Some(v) => v,
                None => return Err(LoadError::InvalidSectionNamesIndex),
            };

            // Load name table.
            let mut names = new_buffer(section.size() as _);

            file.seek(SeekFrom::Start(section.offset())).unwrap();

            if let Err(e) = file.read_exact(&mut names) {
                return Err(LoadError::ReadSectionNamesFailed(e));
            }

            drop(section);

            // Populate section's name.
            for i in 0..sections.len() {
                let section = sections.index_mut(i);
                let name = &names[(section.name_offset() as usize)..];
                let end = match name.iter().position(|&b| b == 0) {
                    Some(v) => v,
                    None => return Err(LoadError::InvalidSectionName(i)),
                };

                section.set_name(&name[..end]);
            }
        };

        Ok(Self {
            file_size,
            entry_addr: e_entry as _,
            segments,
            programs,
            sections,
        })
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn entry_addr(&self) -> usize {
        self.entry_addr
    }

    pub fn segments(&self) -> &[Segment] {
        self.segments.as_slice()
    }

    pub fn programs(&self) -> &[Program] {
        self.programs.as_slice()
    }

    pub fn sections(&self) -> &[Section] {
        self.sections.as_slice()
    }
}

#[derive(Debug)]
pub enum LoadError {
    ReadSelfHeaderFailed(std::io::Error),
    InvalidSelfMagic,
    ReadSelfSegmentHeaderFailed(usize, std::io::Error),
    ReadElfHeaderFailed(std::io::Error),
    InvalidElfMagic,
    UnsupportedBitness,
    UnsupportedEndianness,
    ReadProgramHeaderFailed(usize, std::io::Error),
    ReadSectionHeaderFailed(usize, std::io::Error),
    InvalidSectionNamesIndex,
    ReadSectionNamesFailed(std::io::Error),
    InvalidSectionName(usize),
}

impl Error for LoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadSelfHeaderFailed(e)
            | Self::ReadSelfSegmentHeaderFailed(_, e)
            | Self::ReadElfHeaderFailed(e)
            | Self::ReadProgramHeaderFailed(_, e)
            | Self::ReadSectionHeaderFailed(_, e)
            | Self::ReadSectionNamesFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for LoadError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::ReadSelfHeaderFailed(_) => f.write_str("cannot read SELF header"),
            Self::InvalidSelfMagic => f.write_str("invalid SELF magic"),
            Self::ReadSelfSegmentHeaderFailed(i, _) => {
                write!(f, "cannot read header for SELF segment #{}", i)
            }
            Self::ReadElfHeaderFailed(_) => f.write_str("cannot read ELF header"),
            Self::InvalidElfMagic => f.write_str("invalid ELF magic"),
            Self::UnsupportedBitness => f.write_str("unsupported bitness"),
            Self::UnsupportedEndianness => f.write_str("unsupported endianness"),
            Self::ReadProgramHeaderFailed(i, _) => write!(f, "cannot read program header #{}", i),
            Self::ReadSectionHeaderFailed(i, _) => write!(f, "cannot read section header #{}", i),
            Self::InvalidSectionNamesIndex => f.write_str("invalid index to section names"),
            Self::ReadSectionNamesFailed(_) => f.write_str("cannot read section name table"),
            Self::InvalidSectionName(i) => write!(f, "invalid section name for section #{}", i),
        }
    }
}
