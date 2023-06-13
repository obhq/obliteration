pub use dynamic::*;
pub use program::*;

use bitflags::bitflags;
use byteorder::{ByteOrder, LE};
use std::io::{Read, Seek, SeekFrom};
use thiserror::Error;

mod dynamic;
mod program;

/// The first 8 bytes of SELF file.
pub const SELF_MAGIC: [u8; 8] = [0x4f, 0x15, 0x3d, 0x1d, 0x00, 0x01, 0x01, 0x12];

/// Represents a SELF or ELF file.
///
/// The reason we need to support both SELF and ELF is because every SELF decryptors output ELF.
/// See https://www.psdevwiki.com/ps4/SELF_File_Format for some basic information about SELF.
pub struct Elf<I: Read + Seek> {
    name: String,
    image: I,
    self_data: Option<SelfData>,
    entry_addr: Option<usize>,
    programs: Vec<Program>,
    dynamic_linking: Option<DynamicLinking>,
    twomb_mode: bool,
}

impl<I: Read + Seek> Elf<I> {
    pub fn open<N: Into<String>>(name: N, mut image: I) -> Result<Self, OpenError> {
        // Seek to file header.
        if let Err(e) = image.rewind() {
            return Err(OpenError::SeekFailed(0, e));
        }

        // Read file header.
        let mut hdr = [0u8; 64];

        if let Err(e) = image.read_exact(&mut hdr) {
            return Err(OpenError::ReadHeaderFailed(e));
        }

        // Check if image is SELF.
        let (hdr, offset, self_data) = if hdr.starts_with(&SELF_MAGIC) {
            // Kyty also checking if Category = 0x01 & Program Type = 0x01 & Padding = 0x00.
            // Let's check only magic for now until something is broken.
            if LE::read_u16(&hdr[0x1a..]) != 0x22 {
                return Err(OpenError::InvalidSelfMagic);
            }

            // Load SELF fields.
            let segment_count = LE::read_u16(&hdr[0x18..]) as usize;

            // Seek to the first SELF segment.
            if let Err(e) = image.seek(SeekFrom::Start(32)) {
                return Err(OpenError::SeekFailed(32, e));
            }

            // Load SELF segment headers.
            let mut segments: Vec<SelfSegment> = Vec::with_capacity(segment_count);

            for i in 0..segment_count {
                // Read header.
                let mut hdr = [0u8; 32];

                if let Err(e) = image.read_exact(&mut hdr) {
                    return Err(OpenError::ReadSelfSegmentFailed(i, e));
                }

                // Load fields.
                segments.push(SelfSegment {
                    flags: SelfSegmentFlags::from_bits_retain(LE::read_u64(&hdr)),
                    offset: LE::read_u64(&hdr[8..]),
                    compressed_size: LE::read_u64(&hdr[16..]),
                    decompressed_size: LE::read_u64(&hdr[24..]),
                });
            }

            let self_data = Some(SelfData { segments });

            // Get offset for ELF header.
            let elf_offset = match image.stream_position() {
                Ok(v) => v,
                Err(e) => return Err(OpenError::GetElfOffsetFailed(e)),
            };

            // Read ELF header.
            let mut elf_hdr = [0u8; 64];

            if let Err(e) = image.read_exact(&mut elf_hdr) {
                return Err(OpenError::ReadElfHeaderFailed(e));
            }

            (elf_hdr, elf_offset, self_data)
        } else {
            (hdr, 0, None)
        };

        // Check ELF magic.
        if !hdr.starts_with(&[0x7f, 0x45, 0x4c, 0x46]) {
            return Err(OpenError::InvalidElfMagic);
        }

        // Check ELF type.
        if hdr[0x04] != 2 {
            return Err(OpenError::UnsupportedBitness);
        }

        if hdr[0x05] != 1 {
            return Err(OpenError::UnsupportedEndianness);
        }

        // Load ELF header.
        let e_entry = LE::read_u64(&hdr[0x18..]);
        let e_phoff = offset + 0x40; // PS4 is hard-coded this value.
        let e_phnum = LE::read_u16(&hdr[0x38..]) as usize;

        // Seek to first program header.
        match image.seek(SeekFrom::Start(e_phoff)) {
            Ok(v) => {
                if v != e_phoff {
                    return Err(OpenError::InvalidProgramOffset);
                }
            }
            Err(e) => return Err(OpenError::SeekFailed(e_phoff, e)),
        }

        // Load program headers.
        let mut programs: Vec<Program> = Vec::with_capacity(e_phnum);
        let mut mapbase = u64::MAX;
        let mut mapend = 0;
        let mut twomb_mode = false;
        let mut relro = None;
        let mut exec = None;
        let mut data = None;
        let mut dynamic: Option<(usize, usize)> = None;
        let mut dynlib: Option<(usize, usize)> = None;

        for i in 0..e_phnum {
            // Read header.
            let mut hdr = [0u8; 0x38];

            if let Err(e) = image.read_exact(&mut hdr) {
                return Err(OpenError::ReadProgramHeaderFailed(i, e));
            }

            // Load the header.
            let ty = ProgramType::new(LE::read_u32(&hdr));
            let flags = ProgramFlags::from_bits_retain(LE::read_u32(&hdr[0x04..]));
            let offset = LE::read_u64(&hdr[0x08..]);
            let addr = LE::read_u64(&hdr[0x10..]);
            let file_size = LE::read_u64(&hdr[0x20..]);
            let memory_size = LE::read_u64(&hdr[0x28..]);
            let align = LE::read_u64(&hdr[0x30..]);

            // Process the header.
            match ty {
                ProgramType::PT_LOAD | ProgramType::PT_SCE_RELRO => {
                    // Check offset.
                    if offset > 0xffffffff || offset & 0x3fff != 0 {
                        return Err(OpenError::InvalidOffset(i, ty));
                    }

                    // Check address.
                    if addr & 0x3fff != 0 {
                        return Err(OpenError::InvalidAddr(i, ty));
                    } else if align & 0x3fff != 0 {
                        return Err(OpenError::InvalidAligment(i, ty));
                    }

                    // Check size.
                    if file_size > memory_size {
                        return Err(OpenError::InvalidFileSize(i, ty));
                    } else if memory_size > 0x7fffffff {
                        return Err(OpenError::InvalidMemSize(i, ty));
                    }

                    // Update mapped base.
                    if addr < mapbase {
                        mapbase = addr;
                    }

                    if mapend < Program::align_page(addr + memory_size) {
                        mapend = Program::align_page(addr + memory_size);
                    }

                    // Check if memory size is larger than 2 MB.
                    if memory_size > 0x1fffff {
                        twomb_mode = true;
                    }

                    // Keep index of the header.
                    if ty == ProgramType::PT_SCE_RELRO {
                        relro = Some(i);
                    } else if flags.contains(ProgramFlags::EXECUTE) {
                        exec = Some(i);
                    } else if data.is_none() {
                        data = Some(i);
                    }
                }
                ProgramType::PT_DYNAMIC => dynamic = Some((i, file_size as usize)),
                ProgramType::PT_TLS => {
                    // Check offset.
                    if offset > 0xffffffff {
                        return Err(OpenError::InvalidOffset(i, ty));
                    }

                    // Check size.
                    if file_size > memory_size {
                        return Err(OpenError::InvalidFileSize(i, ty));
                    } else if memory_size > 0x7fffffff {
                        return Err(OpenError::InvalidMemSize(i, ty));
                    }

                    // Check aligment.
                    if align > 32 {
                        return Err(OpenError::InvalidAligment(i, ty));
                    }
                }
                ProgramType::PT_SCE_DYNLIBDATA => dynlib = Some((i, file_size as usize)),
                _ => {}
            }

            programs.push(Program::new(
                ty,
                flags,
                offset,
                addr as usize,
                file_size,
                memory_size as usize,
            ));
        }

        // Check mapable program.
        if mapbase == u64::MAX || mapend == 0 {
            return Err(OpenError::NoMappableProgram);
        }

        // Check PT_SCE_RELRO.
        if let Some(i) = relro {
            let relro = &programs[i];

            if relro.addr() == 0 {
                return Err(OpenError::InvalidRelroAddr);
            } else if relro.memory_size() == 0 {
                return Err(OpenError::InvalidRelroSize);
            }

            // Check if PT_SCE_RELRO follows the executable program.
            if let Some(i) = exec {
                let exec = &programs[i];

                if Program::align_2mb(exec.end() as u64) as usize != relro.addr()
                    && Program::align_page(exec.end() as u64) as usize != relro.addr()
                {
                    return Err(OpenError::InvalidRelroAddr);
                }
            };

            // Check if data follows the PT_SCE_RELRO.
            if let Some(i) = data {
                let data = &programs[i];

                if Program::align_2mb(relro.end() as u64) as usize != data.addr()
                    && Program::align_page(relro.end() as u64) as usize != data.addr()
                {
                    return Err(OpenError::InvalidDataAddr(i));
                }
            }
        }

        let mut elf = Self {
            name: name.into(),
            image,
            self_data,
            entry_addr: match e_entry {
                0 => None,
                v => Some(v as usize),
            },
            programs,
            dynamic_linking: None,
            twomb_mode,
        };

        // Load dynamic linking data.
        if let Some((dynamic_index, dynamic_len)) = dynamic {
            let (dynlib_index, dynlib_len) = match dynlib {
                Some(v) => v,
                None => return Err(OpenError::NoDynlibData),
            };

            // Read PT_DYNAMIC.
            let mut dynamic = vec![0u8; dynamic_len];

            if let Err(e) = elf.read_program(dynamic_index, &mut dynamic) {
                return Err(OpenError::ReadDynamicFailed(e));
            }

            // Read PT_SCE_DYNLIBDATA.
            let mut dynlib = vec![0u8; dynlib_len];

            if let Err(e) = elf.read_program(dynlib_index, &mut dynlib) {
                return Err(OpenError::ReadDynlibDataFailed(e));
            }

            // Parse PT_DYNAMIC & PT_SCE_DYNLIBDATA.
            elf.dynamic_linking = match DynamicLinking::parse(dynamic, dynlib) {
                Ok(v) => Some(v),
                Err(e) => return Err(OpenError::ParseDynamicLinkingFailed(e)),
            };
        } else if dynlib.is_some() {
            return Err(OpenError::NoDynamic);
        }

        Ok(elf)
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn self_segments(&self) -> Option<&[SelfSegment]> {
        self.self_data.as_ref().map(|d| d.segments.as_slice())
    }

    pub fn entry_addr(&self) -> Option<usize> {
        self.entry_addr
    }

    pub fn programs(&self) -> &[Program] {
        self.programs.as_slice()
    }

    pub fn dynamic_linking(&self) -> Option<&DynamicLinking> {
        self.dynamic_linking.as_ref()
    }

    pub fn twomb_mode(&self) -> bool {
        self.twomb_mode
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

        // Seek file to data offset.
        let offset = self.get_program_offset(prog)?;

        match self.image.seek(SeekFrom::Start(offset)) {
            Ok(v) => {
                if v != offset {
                    panic!("File is smaller than {offset} bytes.");
                }
            }
            Err(e) => return Err(ReadProgramError::SeekFailed(offset, e)),
        }

        // Read data.
        if let Err(e) = self.image.read_exact(&mut buf[..len]) {
            return Err(ReadProgramError::ReadFailed(offset, len, e));
        }

        Ok(())
    }

    fn get_program_offset(&self, prog: &Program) -> Result<u64, ReadProgramError> {
        match &self.self_data {
            Some(self_data) => {
                // Find the target segment.
                let offset = prog.offset();
                let len = prog.file_size();

                for (i, seg) in self_data.segments.iter().enumerate() {
                    // Skip if not blocked segment.
                    let flags = seg.flags;

                    if !flags.contains(SelfSegmentFlags::SF_BFLG) {
                        continue;
                    }

                    // Check if the target offset inside the associated program.
                    let prog = &self.programs[flags.program()];

                    if offset >= prog.offset() && offset < prog.offset() + prog.file_size() {
                        // Check if segment supported.
                        if flags.contains(SelfSegmentFlags::SF_ENCR) {
                            return Err(ReadProgramError::EncryptedSegment(i));
                        }

                        if seg.compressed_size != seg.decompressed_size {
                            panic!("Compressed SELF segment is not supported yet.");
                        }

                        if seg.decompressed_size != prog.file_size() {
                            panic!("SELF segment size different than associated program segment is not supported yet.");
                        }

                        // Get data offset.
                        let offset = offset - prog.offset();

                        if offset + len > seg.decompressed_size {
                            panic!("Segment block is smaller than the size specified in program header.");
                        }

                        return Ok(offset + seg.offset);
                    }
                }

                panic!("SELF image is corrupted.");
            }
            None => Ok(prog.offset()),
        }
    }
}

/// Contains data specific for SELF.
struct SelfData {
    segments: Vec<SelfSegment>,
}

/// Represents a SELF segment.
pub struct SelfSegment {
    flags: SelfSegmentFlags,
    offset: u64,
    compressed_size: u64,
    decompressed_size: u64,
}

impl SelfSegment {
    pub fn flags(&self) -> SelfSegmentFlags {
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

bitflags! {
    /// Represents flags of SELF segment.
    #[derive(Clone, Copy)]
    pub struct SelfSegmentFlags: u64 {
        const SF_ORDR = 0x0000000000000001;
        const SF_ENCR = 0x0000000000000002;
        const SF_SIGN = 0x0000000000000004;
        const SF_DFLG = 0x0000000000000008;
        const SF_BFLG = 0x0000000000000800;
    }
}

impl SelfSegmentFlags {
    pub fn program(self) -> usize {
        ((self.bits() >> 20) & 0xfff) as usize
    }
}

/// Represents an error for [`Elf::open()`].
#[derive(Debug, Error)]
pub enum OpenError {
    #[error("cannot seek to offset {0}")]
    SeekFailed(u64, #[source] std::io::Error),

    #[error("cannot read file header")]
    ReadHeaderFailed(#[source] std::io::Error),

    #[error("invalid SELF magic")]
    InvalidSelfMagic,

    #[error("cannot read a header for SELF segment #{0}")]
    ReadSelfSegmentFailed(usize, #[source] std::io::Error),

    #[error("cannot get offset of ELF header")]
    GetElfOffsetFailed(#[source] std::io::Error),

    #[error("cannot read ELF header")]
    ReadElfHeaderFailed(#[source] std::io::Error),

    #[error("invalid ELF magic")]
    InvalidElfMagic,

    #[error("unsupported bitness")]
    UnsupportedBitness,

    #[error("unsupported endianness")]
    UnsupportedEndianness,

    #[error("e_phoff is not valid")]
    InvalidProgramOffset,

    #[error("cannot read program header #{0}")]
    ReadProgramHeaderFailed(usize, #[source] std::io::Error),

    #[error("{1} at program {0} has invalid file offset")]
    InvalidOffset(usize, ProgramType),

    #[error("{1} at program {0} has invalid address")]
    InvalidAddr(usize, ProgramType),

    #[error("{1} at program {0} has invalid aligment")]
    InvalidAligment(usize, ProgramType),

    #[error("{1} at program {0} has invalid file size")]
    InvalidFileSize(usize, ProgramType),

    #[error("{1} at program {0} has invalid memory size")]
    InvalidMemSize(usize, ProgramType),

    #[error("no mappable program")]
    NoMappableProgram,

    #[error("PT_SCE_RELRO has invalid address")]
    InvalidRelroAddr,

    #[error("PT_SCE_RELRO has invalid size")]
    InvalidRelroSize,

    #[error("PT_LOAD at program {0} has invalid address")]
    InvalidDataAddr(usize),

    #[error("no PT_DYNAMIC")]
    NoDynamic,

    #[error("no PT_SCE_DYNLIBDATA")]
    NoDynlibData,

    #[error("cannot read PT_DYNAMIC")]
    ReadDynamicFailed(#[source] ReadProgramError),

    #[error("cannot read PT_SCE_DYNLIBDATA")]
    ReadDynlibDataFailed(#[source] ReadProgramError),

    #[error("cannot parse PT_DYNAMIC and PT_SCE_DYNLIBDATA")]
    ParseDynamicLinkingFailed(#[source] self::dynamic::ParseError),
}

/// Represents an error for [`Elf::read_program()`].
#[derive(Debug, Error)]
pub enum ReadProgramError {
    #[error("invalid program index")]
    InvalidIndex,

    #[error("insufficient buffer (need {0} bytes)")]
    InsufficientBuffer(usize),

    #[error("SELF segment #{0} is encrypted")]
    EncryptedSegment(usize),

    #[error("cannot seek to offset {0:#018x}")]
    SeekFailed(u64, #[source] std::io::Error),

    #[error("cannot read {1} bytes at offset {0:#018x}")]
    ReadFailed(u64, usize, #[source] std::io::Error),
}
