use self::program::Program;
use self::segment::SignedSegment;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use thiserror::Error;
use util::mem::{read_array, read_u16_le, read_u32_le, read_u64_le, read_u8, uninit};

pub mod program;
pub mod segment;

/// Represents a SELF file.
///
/// See https://www.psdevwiki.com/ps4/SELF_File_Format for some basic information.
pub struct SignedElf {
    file: File,
    file_size: u64,
    entry_addr: usize,
    segments: Vec<SignedSegment>,
    programs: Vec<Program>,
}

impl SignedElf {
    pub fn load(file: crate::fs::File) -> Result<Self, LoadError> {
        // Open the file without allocating a virtual file descriptor.
        let mut file = match File::open(file.path()) {
            Ok(v) => v,
            Err(e) => return Err(LoadError::OpenFailed(e)),
        };

        // Read SELF header.
        let mut hdr: [u8; 32] = unsafe { uninit() };

        if let Err(e) = file.read_exact(&mut hdr) {
            return Err(LoadError::ReadSelfHeaderFailed(e));
        }

        let hdr = hdr.as_ptr();

        // Check magic.
        // Kyty also checking if Category = 0x01 & Program Type = 0x01 & Padding = 0x00.
        // Let's check only magic for now until something is broken.
        let magic: [u8; 8] = unsafe { read_array(hdr, 0x00) };
        let unknown = unsafe { read_u16_le(hdr, 0x1a) };

        if magic != [0x4f, 0x15, 0x3d, 0x1d, 0x00, 0x01, 0x01, 0x12] || unknown != 0x22 {
            return Err(LoadError::InvalidSelfMagic);
        }

        // Load SELF fields.
        let file_size = unsafe { read_u64_le(hdr, 0x10) };

        // Load SELF segment headers.
        let segment_count = unsafe { read_u16_le(hdr, 0x18) } as usize;
        let mut segments: Vec<SignedSegment> = Vec::with_capacity(segment_count);

        for i in 0..segment_count {
            // Read header.
            let mut hdr: [u8; 32] = unsafe { uninit() };

            if let Err(e) = file.read_exact(&mut hdr) {
                return Err(LoadError::ReadSelfSegmentHeaderFailed(i, e));
            }

            let hdr = hdr.as_ptr();

            // Load fields.
            let flags = unsafe { read_u64_le(hdr, 0) };
            let offset = unsafe { read_u64_le(hdr, 8) };
            let compressed_size = unsafe { read_u64_le(hdr, 16) };
            let decompressed_size = unsafe { read_u64_le(hdr, 24) };

            segments.push(SignedSegment::new(
                flags.into(),
                offset,
                compressed_size,
                decompressed_size,
            ));
        }

        // Read ELF header.
        let elf_offset = file.stream_position().unwrap();
        let mut hdr: [u8; 64] = unsafe { uninit() };

        if let Err(e) = file.read_exact(&mut hdr) {
            return Err(LoadError::ReadElfHeaderFailed(e));
        }

        let hdr = hdr.as_ptr();

        // Check ELF magic.
        let magic: [u8; 4] = unsafe { read_array(hdr, 0x00) };

        if magic != [0x7f, 0x45, 0x4c, 0x46] {
            return Err(LoadError::InvalidElfMagic);
        }

        // Check ELF type.
        if unsafe { read_u8(hdr, 0x04) } != 2 {
            return Err(LoadError::UnsupportedBitness);
        }

        if unsafe { read_u8(hdr, 0x05) } != 1 {
            return Err(LoadError::UnsupportedEndianness);
        }

        // Load ELF header.
        let e_entry = unsafe { read_u64_le(hdr, 0x18) };
        let e_phoff = unsafe { read_u64_le(hdr, 0x20) };
        let e_phnum = unsafe { read_u16_le(hdr, 0x38) };

        // Load program headers.
        let mut programs: Vec<Program> = Vec::with_capacity(e_phnum as _);

        file.seek(SeekFrom::Start(elf_offset + e_phoff)).unwrap();

        for i in 0..e_phnum {
            // Read header.
            let mut hdr: [u8; 0x38] = unsafe { uninit() };

            if let Err(e) = file.read_exact(&mut hdr) {
                return Err(LoadError::ReadProgramHeaderFailed(i as _, e));
            }

            let hdr = hdr.as_ptr();

            // Load fields.
            let p_type = unsafe { read_u32_le(hdr, 0x00) };
            let p_flags = unsafe { read_u32_le(hdr, 0x04) };
            let p_offset = unsafe { read_u64_le(hdr, 0x08) };
            let p_vaddr = unsafe { read_u64_le(hdr, 0x10) };
            let p_filesz = unsafe { read_u64_le(hdr, 0x20) };
            let p_memsz = unsafe { read_u64_le(hdr, 0x28) };
            let p_align = unsafe { read_u64_le(hdr, 0x30) };

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

        Ok(Self {
            file,
            file_size,
            entry_addr: e_entry as _,
            segments,
            programs,
        })
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn entry_addr(&self) -> usize {
        self.entry_addr
    }

    pub fn segments(&self) -> &[SignedSegment] {
        self.segments.as_slice()
    }

    pub fn programs(&self) -> &[Program] {
        self.programs.as_slice()
    }

    pub fn read_program(&mut self, index: usize, buf: &mut [u8]) -> Result<(), ReadProgramError> {
        // Get target program.
        let prog = match self.programs.get(index) {
            Some(v) => v,
            None => return Err(ReadProgramError::InvalidIndex),
        };

        // Check if buffer is large enough.
        let len = prog.file_size() as usize;

        if buf.len() < len {
            return Err(ReadProgramError::InsufficientBuffer(len));
        }

        // Find the target segment.
        let offset = prog.offset();

        for seg in &self.segments {
            // Skip if not blocked segment.
            let flags = seg.flags();

            if !flags.is_blocked() {
                continue;
            }

            // Check if the target offset inside the associated program.
            let prog = &self.programs[flags.id() as usize];

            if offset >= prog.offset() && offset < prog.offset() + prog.file_size() {
                // Check if segment supported.
                if seg.compressed_size() != seg.decompressed_size() {
                    panic!("Compressed SELF segment is not supported yet.");
                }

                if seg.decompressed_size() != prog.file_size() {
                    panic!("SELF segment size different than associated program segment is not supported yet.");
                }

                // Seek file to data offset.
                let offset = offset - prog.offset();

                if (offset as usize) + len > seg.decompressed_size() as usize {
                    panic!("Segment block is smaller than the size specified in program header.");
                }

                let offset = offset + seg.offset();

                match self.file.seek(SeekFrom::Start(offset)) {
                    Ok(v) => {
                        if v != offset {
                            panic!("File is smaller than {offset} bytes.");
                        }
                    }
                    Err(e) => return Err(ReadProgramError::SeekFailed(offset, e)),
                }

                // Read data.
                if let Err(e) = self.file.read_exact(&mut buf[..len]) {
                    return Err(ReadProgramError::ReadFailed(offset, len, e));
                }

                return Ok(());
            }
        }

        Ok(())
    }
}

/// Represents errors for [`SignedElf::load()`].
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("cannot open SELF file")]
    OpenFailed(#[source] std::io::Error),

    #[error("cannot read SELF header")]
    ReadSelfHeaderFailed(#[source] std::io::Error),

    #[error("invalid SELF magic")]
    InvalidSelfMagic,

    #[error("cannot read header for SELF segment #{0}")]
    ReadSelfSegmentHeaderFailed(usize, #[source] std::io::Error),

    #[error("cannot read ELF header")]
    ReadElfHeaderFailed(#[source] std::io::Error),

    #[error("invalid ELF magic")]
    InvalidElfMagic,

    #[error("unsupported bitness")]
    UnsupportedBitness,

    #[error("unsupported endianness")]
    UnsupportedEndianness,

    #[error("cannot read program header #{0}")]
    ReadProgramHeaderFailed(usize, #[source] std::io::Error),
}

/// Represents errors for [`SignedElf::read_program()`].
#[derive(Debug, Error)]
pub enum ReadProgramError {
    #[error("invalid program index")]
    InvalidIndex,

    #[error("insufficient buffer (need {0} bytes)")]
    InsufficientBuffer(usize),

    #[error("cannot seek to offset {0:#018x}")]
    SeekFailed(u64, #[source] std::io::Error),

    #[error("cannot read {1} bytes at offset {0:#018x}")]
    ReadFailed(u64, usize, #[source] std::io::Error),
}
