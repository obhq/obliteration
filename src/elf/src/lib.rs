pub use dynamic::*;
pub use info::*;
pub use program::*;
pub use reloc::*;
pub use ty::*;

use bitflags::bitflags;
use byteorder::{ByteOrder, LE};
use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;
use thiserror::Error;

mod dynamic;
mod info;
mod program;
mod reloc;
mod ty;

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
    ty: FileType,
    entry_addr: Option<usize>,
    programs: Vec<Program>,
    mapping: Range<usize>,
    code: Option<usize>,
    relro: Option<usize>,
    data: Option<usize>,
    dynamic: Option<usize>,
    dyndata: Option<usize>,
    tls: Option<usize>,
    dynamic_linking: Option<DynamicLinking>,
    proc_param: Option<usize>,
    mod_param: Option<usize>,
    comment: Option<usize>,
    eh: Option<usize>,
    twomb_mode: bool,
    info: Option<FileInfo>,
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
        let e_type = FileType::new(LE::read_u16(&hdr[0x10..]));
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

        // Read program headers.
        let mut data = vec![0u8; e_phnum * 0x38];

        if let Err(e) = image.read_exact(&mut data) {
            return Err(OpenError::ReadProgramHeadersFailed(e));
        }

        // Load program headers.
        let mut elf = Self {
            name: name.into(),
            image,
            self_data,
            ty: e_type,
            entry_addr: match e_entry {
                0 => None,
                v => Some(v as usize),
            },
            programs: Vec::with_capacity(e_phnum),
            mapping: Range {
                start: usize::MAX,
                end: 0,
            },
            code: None,
            relro: None,
            data: None,
            dynamic: None,
            dyndata: None,
            tls: None,
            dynamic_linking: None,
            proc_param: None,
            mod_param: None,
            comment: None,
            eh: None,
            twomb_mode: false,
            info: None,
        };

        for (i, h) in data.chunks_exact(0x38).enumerate() {
            // Load the header.
            let p = Program::new(
                ProgramType::new(LE::read_u32(h)),
                ProgramFlags::from_bits_retain(LE::read_u32(&h[0x04..])),
                LE::read_u64(&h[0x08..]),
                LE::read_u64(&h[0x10..]) as usize,
                LE::read_u64(&h[0x20..]),
                LE::read_u64(&h[0x28..]) as usize,
                LE::read_u64(&h[0x30..]) as usize,
            );

            // Process the header.
            match p.ty() {
                ProgramType::PT_LOAD | ProgramType::PT_SCE_RELRO => elf.process_mappable(i, &p)?,
                ProgramType::PT_DYNAMIC => elf.process_dynamic(i, &p)?,
                ProgramType::PT_TLS => elf.process_tls(i, &p)?,
                ProgramType::PT_SCE_DYNLIBDATA => elf.process_dyndata(i, &p)?,
                ProgramType::PT_SCE_PROCPARAM => elf.proc_param = Some(i),
                ProgramType::PT_SCE_MODULE_PARAM => elf.mod_param = Some(i),
                ProgramType::PT_SCE_COMMENT => elf.process_comment(i, &p)?,
                ProgramType::PT_GNU_EH_FRAME => elf.process_eh(i, &p)?,
                _ => {}
            }

            elf.programs.push(p);
        }

        // Check mapping range.
        if elf.mapping.start == usize::MAX || elf.mapping.end == 0 {
            return Err(OpenError::NoMappableProgram);
        }

        // Check dynamic linking.
        if let Some(i) = elf.dynamic {
            let dynamic = &elf.programs[i];

            if dynamic.file_size() == 0 {
                return Err(OpenError::InvalidDynamic);
            }

            let mut dynoff: usize = dynamic.offset().try_into().unwrap();
            let dynsize: usize = dynamic.file_size().try_into().unwrap();

            // Read PT_DYNAMIC.
            let mut dynamic = vec![0u8; dynamic.file_size() as usize];

            if let Err(e) = elf.read_program(i, &mut dynamic) {
                return Err(OpenError::ReadDynamicFailed(e));
            }

            // Check dynamic data.
            let i = elf.dyndata.ok_or(OpenError::NoDynData)?;
            let dyndata = &elf.programs[i];

            if dyndata.file_size() == 0 {
                return Err(OpenError::InvalidDynData);
            }

            // Adjust dynamic offset inside the dynamic data. It looks weird but this is how Sony
            // actually did.
            dynoff -= dyndata.offset() as usize;

            // Read PT_SCE_DYNLIBDATA.
            let mut dyndata = vec![0u8; dyndata.file_size() as usize];

            if let Err(e) = elf.read_program(i, &mut dyndata) {
                return Err(OpenError::ReadDynDataFailed(e));
            }

            // Read PT_SCE_COMMENT.
            let comment = if let Some(i) = elf.comment {
                let mut buf = vec![0u8; elf.programs[i].file_size() as usize];

                if elf.read_program(i, &mut buf).is_err() {
                    // This is not an error on the PS4.
                    Vec::new()
                } else {
                    buf
                }
            } else {
                Vec::new()
            };

            // Load info.
            elf.info = match FileInfo::parse(dyndata.clone(), comment, dynoff, dynsize) {
                Ok(v) => Some(v),
                Err(e) => return Err(OpenError::ParseFileInfoFailed(e)),
            };

            // Parse PT_DYNAMIC & PT_SCE_DYNLIBDATA.
            elf.dynamic_linking = match DynamicLinking::parse(dynamic, dyndata) {
                Ok(v) => Some(v),
                Err(e) => return Err(OpenError::ParseDynamicLinkingFailed(e)),
            };
        }

        // Check PT_SCE_RELRO.
        if let Some(i) = elf.relro {
            let relro = &elf.programs[i];

            if relro.addr() == 0 {
                return Err(OpenError::InvalidRelroAddr);
            } else if relro.memory_size() == 0 {
                return Err(OpenError::InvalidRelroSize);
            }

            // Check if PT_SCE_RELRO follows the code.
            if let Some(i) = elf.code {
                let code = &elf.programs[i];

                if Program::align_2mb(code.end() as u64) as usize != relro.addr()
                    && Program::align_page(code.end() as u64) as usize != relro.addr()
                {
                    return Err(OpenError::InvalidRelroAddr);
                }
            };

            // Check if data follows the PT_SCE_RELRO.
            if let Some(i) = elf.data {
                let data = &elf.programs[i];

                if Program::align_2mb(relro.end() as u64) as usize != data.addr()
                    && Program::align_page(relro.end() as u64) as usize != data.addr()
                {
                    return Err(OpenError::InvalidDataAddr(i));
                }
            }
        }

        Ok(elf)
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn self_segments(&self) -> Option<&[SelfSegment]> {
        self.self_data.as_ref().map(|d| d.segments.as_slice())
    }

    pub fn ty(&self) -> FileType {
        self.ty
    }

    pub fn entry_addr(&self) -> Option<usize> {
        self.entry_addr
    }

    pub fn programs(&self) -> &[Program] {
        self.programs.as_slice()
    }

    pub fn dynamic(&self) -> Option<usize> {
        self.dynamic
    }

    pub fn tls(&self) -> Option<usize> {
        self.tls
    }

    pub fn dynamic_linking(&self) -> Option<&DynamicLinking> {
        self.dynamic_linking.as_ref()
    }

    pub fn proc_param(&self) -> Option<usize> {
        self.proc_param
    }

    pub fn mod_param(&self) -> Option<usize> {
        self.mod_param
    }

    pub fn comment(&self) -> Option<usize> {
        self.comment
    }

    pub fn eh(&self) -> Option<usize> {
        self.eh
    }

    pub fn twomb_mode(&self) -> bool {
        self.twomb_mode
    }

    /// Only available on a dynamic module.
    ///
    /// See `dynlib_proc_initialize_step1` and `self_load_shared_object` for a reference.
    pub fn info(&self) -> Option<&FileInfo> {
        self.info.as_ref()
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

        // Get program offset.
        let offset = match &self.self_data {
            Some(v) => self.get_self_program(v, prog)?,
            None => prog.offset(),
        };

        // Seek file to data offset.
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

    fn process_mappable(&mut self, index: usize, prog: &Program) -> Result<(), OpenError> {
        // Check offset.
        let ty = prog.ty();
        let offset = prog.offset();

        if offset > 0xffffffff || offset & 0x3fff != 0 {
            return Err(OpenError::InvalidOffset(index, ty));
        }

        // Check address.
        let addr = prog.addr();

        if addr & 0x3fff != 0 {
            return Err(OpenError::InvalidAddr(index, ty));
        } else if prog.alignment() & 0x3fff != 0 {
            return Err(OpenError::InvalidAligment(index, ty));
        }

        // Check size.
        let memory_size = prog.memory_size();

        if (prog.file_size() as usize) > memory_size {
            return Err(OpenError::InvalidFileSize(index, ty));
        } else if memory_size > 0x7fffffff {
            return Err(OpenError::InvalidMemSize(index, ty));
        }

        // Update mapping range.
        let end = Program::align_page((addr + memory_size) as u64) as usize;

        if addr < self.mapping.start {
            self.mapping.start = addr;
        }

        if self.mapping.end < end {
            self.mapping.end = end;
        }

        // Check if memory size is larger than 2 MB.
        if memory_size > 0x1fffff {
            self.twomb_mode = true;
        }

        // Keep index of the header.
        if ty == ProgramType::PT_SCE_RELRO {
            self.relro = Some(index);
        } else if prog.flags().contains(ProgramFlags::EXECUTE) {
            self.code = Some(index);
        } else if self.data.is_none() {
            self.data = Some(index);
        }

        Ok(())
    }

    fn process_dynamic(&mut self, index: usize, prog: &Program) -> Result<(), OpenError> {
        // Check offset.
        let ty = prog.ty();
        let offset = prog.offset();

        if offset > 0xffffffff {
            return Err(OpenError::InvalidOffset(index, ty));
        }

        // Check size.
        let memory_size = prog.memory_size();

        if (prog.file_size() as usize) > memory_size {
            return Err(OpenError::InvalidFileSize(index, ty));
        } else if memory_size > 0x7fffffff {
            return Err(OpenError::InvalidMemSize(index, ty));
        }

        self.dynamic = Some(index);

        Ok(())
    }

    fn process_tls(&mut self, index: usize, prog: &Program) -> Result<(), OpenError> {
        // Check offset.
        let ty = prog.ty();
        let offset = prog.offset();

        if offset > 0xffffffff {
            return Err(OpenError::InvalidOffset(index, ty));
        }

        // Check size.
        let memory_size = prog.memory_size();

        if (prog.file_size() as usize) > memory_size {
            return Err(OpenError::InvalidFileSize(index, ty));
        } else if memory_size > 0x7fffffff {
            return Err(OpenError::InvalidMemSize(index, ty));
        }

        // Check aligment.
        if prog.alignment() > 32 {
            return Err(OpenError::InvalidAligment(index, ty));
        }

        self.tls = Some(index);

        Ok(())
    }

    fn process_dyndata(&mut self, index: usize, prog: &Program) -> Result<(), OpenError> {
        // Check offset.
        let ty = prog.ty();
        let offset = prog.offset();

        if offset > 0xffffffff {
            return Err(OpenError::InvalidOffset(index, ty));
        }

        // Check size.
        if prog.file_size() > 0x7fffffff {
            return Err(OpenError::InvalidFileSize(index, ty));
        } else if prog.memory_size() != 0 {
            return Err(OpenError::InvalidMemSize(index, ty));
        }

        self.dyndata = Some(index);

        Ok(())
    }

    fn process_comment(&mut self, index: usize, prog: &Program) -> Result<(), OpenError> {
        // Check offset.
        let ty = prog.ty();
        let offset = prog.offset();

        if offset > 0xffffffff {
            return Err(OpenError::InvalidOffset(index, ty));
        }

        // Check size.
        if prog.file_size() > 0x7fffffff {
            return Err(OpenError::InvalidFileSize(index, ty));
        } else if prog.memory_size() != 0 {
            return Err(OpenError::InvalidMemSize(index, ty));
        }

        self.comment = Some(index);

        Ok(())
    }

    fn process_eh(&mut self, index: usize, prog: &Program) -> Result<(), OpenError> {
        // Check offset.
        let ty = prog.ty();
        let offset = prog.offset();

        if offset > 0xffffffff {
            return Err(OpenError::InvalidOffset(index, ty));
        }

        // Check size.
        let memory_size = prog.memory_size();

        if (prog.file_size() as usize) > memory_size {
            return Err(OpenError::InvalidFileSize(index, ty));
        } else if memory_size > 0x7fffffff {
            return Err(OpenError::InvalidMemSize(index, ty));
        }

        self.eh = Some(index);

        Ok(())
    }

    fn get_self_program(&self, data: &SelfData, prog: &Program) -> Result<u64, ReadProgramError> {
        // Find the target segment.
        let offset = prog.offset();
        let len = prog.file_size();

        for (i, seg) in data.segments.iter().enumerate() {
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

    #[error("cannot read program headers")]
    ReadProgramHeadersFailed(#[source] std::io::Error),

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

    #[error("PT_DYNAMIC is not valid")]
    InvalidDynamic,

    #[error("cannot read PT_DYNAMIC")]
    ReadDynamicFailed(#[source] ReadProgramError),

    #[error("no PT_SCE_DYNLIBDATA")]
    NoDynData,

    #[error("PT_SCE_DYNLIBDATA is not valid")]
    InvalidDynData,

    #[error("cannot read PT_SCE_DYNLIBDATA")]
    ReadDynDataFailed(#[source] ReadProgramError),

    #[error("cannot parse file info")]
    ParseFileInfoFailed(#[source] FileInfoError),

    #[error("cannot parse PT_DYNAMIC and PT_SCE_DYNLIBDATA")]
    ParseDynamicLinkingFailed(#[source] self::dynamic::ParseError),

    #[error("PT_SCE_RELRO has invalid address")]
    InvalidRelroAddr,

    #[error("PT_SCE_RELRO has invalid size")]
    InvalidRelroSize,

    #[error("PT_LOAD at program {0} has invalid address")]
    InvalidDataAddr(usize),
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
