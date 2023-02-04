use self::dynamic::DynamicLinking;
#[cfg(target_arch = "aarch64")]
use self::recompiler::aarch64::Aarch64Emitter;
#[cfg(target_arch = "x86_64")]
use self::recompiler::x64::X64Emitter;
use self::recompiler::{NativeCode, Recompiler};
use super::Process;
use crate::elf::program::{Program, ProgramFlags, ProgramType};
use crate::elf::SignedElf;
#[allow(unused_imports)]
use std::env::consts::ARCH;
use std::io::Write;
use std::mem::transmute;
use std::path::PathBuf;
use thiserror::Error;
use util::mem::new_buffer;

pub mod dynamic;
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
    pub fn load(
        proc: *mut Process,
        mut elf: SignedElf,
        debug: DebugOpts,
    ) -> Result<Self, LoadError> {
        // FIXME: Remove this temporary variable.
        let programs: Vec<Program> = elf.programs().to_vec();

        // Get size of memory for mapping executable.
        let mut mapped_size = 0;

        for prog in &programs {
            if prog.ty() != ProgramType::PT_LOAD && prog.ty() != ProgramType::PT_SCE_RELRO {
                continue;
            }

            let end = prog.virtual_addr() + prog.aligned_size();

            if end > mapped_size {
                mapped_size = end;
            }
        }

        // Load program segments.
        let mut segments: Vec<Segment> = Vec::new();
        let mut mapped: Vec<u8> = vec![0; mapped_size];
        let mut dynamic_linking: Vec<u8> = Vec::new();
        let mut dynlib_data: Vec<u8> = Vec::new();
        let base: usize = mapped.as_ptr() as usize;

        for i in 0..programs.len() {
            let prog = &programs[i];

            match prog.ty() {
                ProgramType::PT_LOAD | ProgramType::PT_SCE_RELRO => {
                    let addr = prog.virtual_addr();
                    let base = base + addr;
                    let to = &mut mapped[addr..(addr + prog.file_size() as usize)];

                    if let Err(e) = elf.read_program(i, to) {
                        return Err(LoadError::LoadProgramFailed(i, e));
                    }

                    segments.push(Segment {
                        start: base,
                        end: base + prog.aligned_size(),
                        flags: prog.flags(),
                    });
                }
                ProgramType::PT_DYNAMIC => {
                    dynamic_linking = new_buffer(prog.file_size() as _);

                    if let Err(e) = elf.read_program(i, &mut dynamic_linking) {
                        return Err(LoadError::LoadProgramFailed(i, e));
                    }
                }
                ProgramType::PT_SCE_DYNLIBDATA => {
                    dynlib_data = new_buffer(prog.file_size() as _);

                    if let Err(e) = elf.read_program(i, &mut dynlib_data) {
                        return Err(LoadError::LoadProgramFailed(i, e));
                    }
                }
                _ => continue,
            }
        }

        // Parse dynamic linking info.
        let dl = match DynamicLinking::parse(&dynamic_linking, &dynlib_data) {
            Ok(v) => v,
            Err(e) => return Err(LoadError::ParseDynamicLinkingFailed(e)),
        };

        if dl.relaent() != 24 {
            // sizeof(Elf64_Rela)
            return Err(LoadError::InvalidRelaent);
        } else if dl.syment() != 24 {
            // sizeof(Elf64_Sym)
            return Err(LoadError::InvalidSyment);
        } else if dl.pltrel() != DynamicLinking::DT_RELA as _ {
            return Err(LoadError::InvalidPltrel);
        }

        // Dump mapped.
        match std::fs::File::create(&debug.original_mapped_dump) {
            Ok(mut v) => {
                if let Err(e) = v.write_all(&mapped) {
                    return Err(LoadError::WriteOriginalMappedDumpFailed(
                        debug.original_mapped_dump,
                        e,
                    ));
                }
            }
            Err(e) => {
                return Err(LoadError::CreateOriginalMappedDumpFailed(
                    debug.original_mapped_dump,
                    e,
                ));
            }
        }

        // Setup recompiler.
        #[cfg(target_arch = "x86_64")]
        let recompiler = X64Emitter::new(proc, &mapped, segments);

        #[cfg(target_arch = "aarch64")]
        let recompiler = Aarch64Emitter::new(proc, &mapped, segments);

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
}

pub(super) struct DebugOpts {
    pub original_mapped_dump: PathBuf,
}

#[cfg(target_arch = "x86_64")]
pub(super) type EntryPoint = extern "sysv64" fn(*mut Arg, extern "sysv64" fn());

#[cfg(not(target_arch = "x86_64"))]
pub(super) type EntryPoint = extern "C" fn(*mut Arg, extern "C" fn());

#[repr(C)]
pub(super) struct Arg {
    pub argc: i32,
    pub argv: *mut *mut u8,
}

pub struct Segment {
    start: usize,
    end: usize, // Pass the last byte.
    flags: ProgramFlags,
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("cannot read program #{0}")]
    LoadProgramFailed(usize, #[source] crate::elf::ReadProgramError),

    #[error("cannot parse dynamic linking information")]
    ParseDynamicLinkingFailed(#[source] dynamic::ParseError),

    #[error("dynamic linking entry DT_RELAENT or DT_SCE_RELAENT has invalid value")]
    InvalidRelaent,

    #[error("dynamic linking entry DT_SYMENT or DT_SCE_SYMENT has invalid value")]
    InvalidSyment,

    #[error("dynamic linking entry DT_PLTREL or DT_SCE_PLTREL has value other than DT_RELA")]
    InvalidPltrel,

    #[error("cannot create {0} to dump mapped SELF")]
    CreateOriginalMappedDumpFailed(PathBuf, #[source] std::io::Error),

    #[error("cannot write mapped SELF to {0}")]
    WriteOriginalMappedDumpFailed(PathBuf, #[source] std::io::Error),

    #[error("cannot recompile executable")]
    RecompileFailed(#[source] recompiler::RunError),
}
