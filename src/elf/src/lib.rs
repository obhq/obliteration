use self::dynamic::DynamicLinking;
use bitflags::bitflags;
use byteorder::{ByteOrder, LE};
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, SeekFrom};
use thiserror::Error;

pub mod dynamic;

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
    entry_addr: usize,
    programs: Vec<Program>,
    dynamic_linking: Option<DynamicLinking>,
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
                    flags: SelfSegmentFlags {
                        bits: LE::read_u64(&hdr),
                    },
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
        let e_phoff = LE::read_u64(&hdr[0x20..]) + offset;
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
        let mut dynamic: Option<(usize, usize)> = None;
        let mut dynlib: Option<(usize, usize)> = None;

        for i in 0..e_phnum {
            // Read header.
            let mut hdr = [0u8; 0x38];

            if let Err(e) = image.read_exact(&mut hdr) {
                return Err(OpenError::ReadProgramHeaderFailed(i, e));
            }

            // Load fields.
            let prog = Program {
                ty: ProgramType(LE::read_u32(&hdr)),
                flags: match ProgramFlags::from_bits(LE::read_u32(&hdr[0x04..])) {
                    Some(v) => v,
                    None => return Err(OpenError::UnknownProgramFlags(i)),
                },
                offset: LE::read_u64(&hdr[0x08..]),
                addr: LE::read_u64(&hdr[0x10..]) as usize,
                file_size: LE::read_u64(&hdr[0x20..]),
                memory_size: LE::read_u64(&hdr[0x28..]) as usize,
                aligment: LE::read_u64(&hdr[0x30..]) as usize,
            };

            match prog.ty {
                ProgramType::PT_DYNAMIC => dynamic = Some((i, prog.file_size as usize)),
                ProgramType::PT_SCE_DYNLIBDATA => dynlib = Some((i, prog.file_size as usize)),
                _ => {}
            }

            programs.push(prog);
        }

        let mut elf = Self {
            name: name.into(),
            image,
            self_data,
            entry_addr: e_entry as usize,
            programs,
            dynamic_linking: None,
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
            elf.dynamic_linking = match DynamicLinking::parse(&dynamic, &dynlib) {
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

    pub fn entry_addr(&self) -> usize {
        self.entry_addr
    }

    pub fn programs(&self) -> &[Program] {
        self.programs.as_slice()
    }

    pub fn dynamic_linking(&self) -> Option<&DynamicLinking> {
        self.dynamic_linking.as_ref()
    }

    pub fn read_program(&mut self, index: usize, buf: &mut [u8]) -> Result<(), ReadProgramError> {
        // Get target program.
        let prog = match self.programs.get(index) {
            Some(v) => v,
            None => return Err(ReadProgramError::InvalidIndex),
        };

        // Check if buffer is large enough.
        let len = prog.file_size as usize;

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
                let offset = prog.offset;
                let len = prog.file_size;

                for (i, seg) in self_data.segments.iter().enumerate() {
                    // Skip if not blocked segment.
                    let flags = seg.flags;

                    if !flags.contains(SelfSegmentFlags::SF_BFLG) {
                        continue;
                    }

                    // Check if the target offset inside the associated program.
                    let prog = &self.programs[flags.program()];

                    if offset >= prog.offset && offset < prog.offset + prog.file_size {
                        // Check if segment supported.
                        if flags.contains(SelfSegmentFlags::SF_ENCR) {
                            return Err(ReadProgramError::EncryptedSegment(i));
                        }

                        if seg.compressed_size != seg.decompressed_size {
                            panic!("Compressed SELF segment is not supported yet.");
                        }

                        if seg.decompressed_size != prog.file_size {
                            panic!("SELF segment size different than associated program segment is not supported yet.");
                        }

                        // Get data offset.
                        let offset = offset - prog.offset;

                        if offset + len > seg.decompressed_size {
                            panic!("Segment block is smaller than the size specified in program header.");
                        }

                        return Ok(offset + seg.offset);
                    }
                }

                panic!("SELF image is corrupted.");
            }
            None => Ok(prog.offset),
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
        ((self.bits >> 20) & 0xfff) as usize
    }
}

/// Contains information for each ELF program.
pub struct Program {
    ty: ProgramType,
    flags: ProgramFlags,
    offset: u64,
    addr: usize,
    file_size: u64,
    memory_size: usize,
    aligment: usize,
}

impl Program {
    pub fn ty(&self) -> ProgramType {
        self.ty
    }

    pub fn flags(&self) -> ProgramFlags {
        self.flags
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn addr(&self) -> usize {
        self.addr
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn memory_size(&self) -> usize {
        self.memory_size
    }

    pub fn aligment(&self) -> usize {
        self.aligment
    }

    pub fn aligned_size(&self) -> usize {
        if self.aligment != 0 {
            // FIXME: Refactor this for readability.
            (self.memory_size + (self.aligment - 1)) & !(self.aligment - 1)
        } else {
            self.memory_size
        }
    }
}

/// Represents type of an ELF program.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ProgramType(u32);

impl ProgramType {
    pub const PT_LOAD: ProgramType = ProgramType(0x00000001);
    pub const PT_DYNAMIC: ProgramType = ProgramType(0x00000002);
    pub const PT_INTERP: ProgramType = ProgramType(0x00000003);
    pub const PT_TLS: ProgramType = ProgramType(0x00000007);
    pub const PT_SCE_DYNLIBDATA: ProgramType = ProgramType(0x61000000);
    pub const PT_SCE_PROCPARAM: ProgramType = ProgramType(0x61000001);
    pub const PT_SCE_MODULE_PARAM: ProgramType = ProgramType(0x61000002);
    pub const PT_SCE_RELRO: ProgramType = ProgramType(0x61000010);
    pub const PT_SCE_COMMENT: ProgramType = ProgramType(0x6fffff00);
    pub const PT_SCE_VERSION: ProgramType = ProgramType(0x6fffff01);
    pub const PT_GNU_EH_FRAME: ProgramType = ProgramType(0x6474e550);
}

impl Display for ProgramType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::PT_LOAD => f.write_str("PT_LOAD"),
            Self::PT_DYNAMIC => f.write_str("PT_DYNAMIC"),
            Self::PT_INTERP => f.write_str("PT_INTERP"),
            Self::PT_TLS => f.write_str("PT_TLS"),
            Self::PT_SCE_DYNLIBDATA => f.write_str("PT_SCE_DYNLIBDATA"),
            Self::PT_SCE_PROCPARAM => f.write_str("PT_SCE_PROCPARAM"),
            Self::PT_SCE_MODULE_PARAM => f.write_str("PT_SCE_MODULE_PARAM"),
            Self::PT_SCE_RELRO => f.write_str("PT_SCE_RELRO"),
            Self::PT_SCE_COMMENT => f.write_str("PT_SCE_COMMENT"),
            Self::PT_SCE_VERSION => f.write_str("PT_SCE_VERSION"),
            Self::PT_GNU_EH_FRAME => f.write_str("PT_GNU_EH_FRAME"),
            t => write!(f, "{:#010x}", t.0),
        }
    }
}

bitflags! {
    /// Represents flags for an ELF program.
    ///
    /// The values was taken from
    /// https://github.com/freebsd/freebsd-src/blob/main/sys/sys/elf_common.h.
    pub struct ProgramFlags: u32 {
        const EXECUTE = 0x00000001;
        const WRITE = 0x00000002;
        const READ = 0x00000004;
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

    #[error("program #{0} has an unknown flags")]
    UnknownProgramFlags(usize),

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
