use self::recompiler::{NativeCode, Recompiler};
use super::Process;
use crate::elf::program::ProgramType;
use crate::elf::SignedElf;
use crate::fs::file::File;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, SeekFrom};
use std::mem::transmute;
use util::mem::new_buffer;

pub mod recompiler;

#[allow(dead_code)]
pub(super) struct Module {
    entry: EntryPoint,
    recompiled: NativeCode,

    // The reason we need to keep the original mapped SELF is because the recompiled code does not
    // copy any referenced data.
    mapped: Vec<u8>,
}

impl Module {
    pub fn load(proc: *mut Process, elf: SignedElf, mut file: File) -> Result<Self, LoadError> {
        // Get size of memory for mapping executable.
        let mut mapped_size = 0;

        for prog in elf.programs() {
            if prog.ty() != ProgramType::PT_LOAD && prog.ty() != ProgramType::PT_SCE_RELRO {
                continue;
            }

            let end = prog.virtual_addr() + prog.aligned_size();

            if end > mapped_size {
                mapped_size = end;
            }
        }

        // Load program segments.
        let mut mapped: Vec<u8> = vec![0; mapped_size];
        let mut dynamic: Vec<u8> = Vec::new();
        let mut dynamic_data: Vec<u8> = Vec::new();

        for prog in elf.programs() {
            let offset = prog.offset();

            match prog.ty() {
                ProgramType::PT_LOAD | ProgramType::PT_SCE_RELRO => {
                    let addr = prog.virtual_addr();
                    let to = &mut mapped[addr..(addr + prog.file_size() as usize)];

                    Self::load_program_segment(&mut file, &elf, offset, to)?;
                }
                ProgramType::PT_DYNAMIC => {
                    dynamic = new_buffer(prog.file_size() as _);

                    Self::load_program_segment(&mut file, &elf, offset, &mut dynamic)?;
                }
                ProgramType::PT_SCE_DYNLIBDATA => {
                    dynamic_data = new_buffer(prog.file_size() as _);

                    Self::load_program_segment(&mut file, &elf, offset, &mut dynamic_data)?;
                }
                _ => continue,
            }
        }

        // Setup recompiler.
        let recompiler = Recompiler::new(&mapped, proc);

        // Recompile module.
        let (entry, recompiled) = match recompiler.run(&[elf.entry_addr()]) {
            Ok((n, e)) => (unsafe { transmute(e[0]) }, n),
            Err(e) => return Err(LoadError::RecompileFailed(e)),
        };

        Ok(Self {
            entry,
            recompiled,
            mapped,
        })
    }

    pub fn entry(&self) -> EntryPoint {
        self.entry
    }

    // FIXME: Refactor this because the logic does not make sense.
    fn load_program_segment(
        bin: &mut File,
        elf: &SignedElf,
        offset: u64,
        to: &mut [u8],
    ) -> Result<(), LoadError> {
        for (i, seg) in elf.segments().iter().enumerate() {
            let flags = seg.flags();

            if !flags.is_blocked() {
                continue;
            }

            let prog = match elf.programs().get(flags.id() as usize) {
                Some(v) => v,
                None => return Err(LoadError::InvalidSelfSegmentId(i)),
            };

            if offset >= prog.offset() && offset < prog.offset() + prog.file_size() {
                if seg.compressed_size() != seg.decompressed_size() {
                    panic!("Compressed SELF segment is not supported yet.");
                }

                if seg.decompressed_size() != prog.file_size() {
                    panic!("SELF segment size different than associated program segment is not supported yet.");
                }

                let offset = offset - prog.offset();

                if (offset as usize) + to.len() > seg.decompressed_size() as usize {
                    panic!("Segment block is smaller than the size specified in program header.");
                }

                bin.seek(SeekFrom::Start(offset + seg.offset())).unwrap();
                bin.read_exact(to).unwrap();

                return Ok(());
            }
        }

        if (bin.len().unwrap() as usize) - (elf.file_size() as usize) == to.len() {
            bin.seek(SeekFrom::Start(elf.file_size())).unwrap();
            bin.read_exact(to).unwrap();

            return Ok(());
        }

        panic!("missing self segment");
    }
}

pub(super) type EntryPoint = extern "sysv64" fn(*mut Arg, extern "sysv64" fn());

#[repr(C)]
pub(super) struct Arg {
    pub argc: i32,
    pub argv: *mut *mut u8,
}

#[derive(Debug)]
pub enum LoadError {
    InvalidSelfSegmentId(usize),
    RecompileFailed(recompiler::RunError),
}

impl Error for LoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::RecompileFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for LoadError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidSelfSegmentId(i) => {
                write!(f, "invalid identifier for SELF segment #{}", i)
            }
            Self::RecompileFailed(_) => f.write_str("cannot recompile executable"),
        }
    }
}
