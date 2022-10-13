use self::recompiler::{NativeCode, Recompiler};
use crate::elf::program::ProgramType;
use crate::elf::SignedElf;
use crate::fs::file::File;
use crate::info;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, SeekFrom};
use std::mem::transmute;
use std::os::raw::c_int;
use std::pin::Pin;
use std::ptr::null_mut;
use util::mem::{new_buffer, uninit};

pub mod recompiler;

/// This struct and its data is highly unsafe. **So make sure you understand what it does before
/// editing any code here.**
pub struct Process {
    id: c_int,
    entry: extern "sysv64" fn(*mut Arg, extern "sysv64" fn()),

    // This field hold a recompiled code that is executing by host CPU and an original mapped SELF
    // so we need to keep it and drop it as a last field. The reason we need to keep the original
    // mapped SELF is because the recompiled code does not copy any referenced data.
    #[allow(dead_code)]
    modules: Vec<(Vec<u8>, NativeCode)>,
}

impl Process {
    pub fn load(elf: SignedElf, mut file: File) -> Result<Pin<Box<Self>>, LoadError> {
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
        let mut proc = Box::pin(Self {
            id: 1,
            entry: uninit(),
            modules: Vec::new(),
        });

        let recompiler = Recompiler::new(&mapped, &mut *proc);

        // Recompile executable.
        proc.entry = match recompiler.run(&[elf.entry_addr()]) {
            Ok((n, e)) => {
                proc.modules.push((mapped, n));
                unsafe { transmute(e[0]) }
            }
            Err(e) => return Err(LoadError::RecompileFailed(e)),
        };

        Ok(proc)
    }

    pub fn run(&mut self) -> Result<i32, RunError> {
        // TODO: Check how the actual binary read its argument.
        // Setup arguments.
        let mut argv: Vec<*mut u8> = Vec::new();
        let mut arg1 = b"prog\0".to_vec();

        argv.push(arg1.as_mut_ptr());
        argv.push(null_mut());

        // Invoke entry point.
        let mut arg = Arg {
            argc: (argv.len() as i32) - 1,
            argv: argv.as_mut_ptr(),
        };

        (self.entry)(&mut arg, Self::exit);

        Ok(0)
    }

    extern "sysv64" fn exit() {
        // TODO: What should we do here?
    }

    extern "sysv64" fn handle_ud2(&mut self, addr: usize) -> ! {
        info!(
            self.id,
            "Process exited with UD2 instruction from {:#018x}.", addr
        );

        // FIXME: Return to "run" without stack unwinding on Windows.
        std::process::exit(0);
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

#[repr(C)]
struct Arg {
    argc: i32,
    argv: *mut *mut u8,
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
