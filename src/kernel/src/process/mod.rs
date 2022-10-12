use self::recompiler::{NativeCode, Recompiler};
use crate::exe::program::ProgramType;
use crate::exe::Executable;
use crate::fs::file::File;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, SeekFrom};
use std::mem::transmute;
use util::mem::new_buffer;

pub mod recompiler;

pub struct Process {
    entry: extern "sysv64" fn(),

    // This field hold a recompiled code that is executing by host CPU and an original mapped SELF
    // so we need to keep it and drop it as a last field. The reason we need to keep the original
    // mapped SELF is because the recompiled code does not copy any referenced data.
    #[allow(dead_code)]
    modules: Vec<(Vec<u8>, NativeCode)>,
}

impl Process {
    pub fn load(exe: Executable, mut file: File) -> Result<Self, LoadError> {
        // Get size of memory for mapping executable.
        let mut mapped_size = 0;

        for prog in exe.programs() {
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

        for prog in exe.programs() {
            let offset = prog.offset();

            match prog.ty() {
                ProgramType::PT_LOAD | ProgramType::PT_SCE_RELRO => {
                    let addr = prog.virtual_addr();
                    let to = &mut mapped[addr..(addr + prog.file_size() as usize)];

                    Self::load_program_segment(&mut file, &exe, offset, to)?;
                }
                ProgramType::PT_DYNAMIC => {
                    dynamic = new_buffer(prog.file_size() as _);

                    Self::load_program_segment(&mut file, &exe, offset, &mut dynamic)?;
                }
                ProgramType::PT_SCE_DYNLIBDATA => {
                    dynamic_data = new_buffer(prog.file_size() as _);

                    Self::load_program_segment(&mut file, &exe, offset, &mut dynamic_data)?;
                }
                _ => continue,
            }
        }

        // Recompile executable.
        let mut modules: Vec<(Vec<u8>, NativeCode)> = Vec::new();
        let recompiler = Recompiler::new(&mapped);
        let entry = match recompiler.run(&[exe.entry_addr()]) {
            Ok((n, e)) => {
                modules.push((mapped, n));
                e[0]
            }
            Err(e) => return Err(LoadError::RecompileFailed(e)),
        };

        Ok(Self {
            entry: unsafe { transmute(entry) },
            modules,
        })
    }

    pub fn run(&mut self) -> Result<i32, RunError> {
        (self.entry)();

        Ok(0)
    }

    // FIXME: Refactor this because the logic does not make sense.
    fn load_program_segment(
        bin: &mut File,
        exe: &Executable,
        offset: u64,
        to: &mut [u8],
    ) -> Result<(), LoadError> {
        for (i, seg) in exe.segments().iter().enumerate() {
            let flags = seg.flags();

            if !flags.is_blocked() {
                continue;
            }

            let prog = match exe.programs().get(flags.id() as usize) {
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

        if (bin.len().unwrap() as usize) - (exe.file_size() as usize) == to.len() {
            bin.seek(SeekFrom::Start(exe.file_size())).unwrap();
            bin.read_exact(to).unwrap();

            return Ok(());
        }

        panic!("missing self segment");
    }
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

#[derive(Debug)]
pub enum RunError {}

impl Error for RunError {}

impl Display for RunError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        Ok(())
    }
}
