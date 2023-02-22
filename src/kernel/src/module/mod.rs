use crate::memory::MemoryManager;
use elf::{Elf, ProgramFlags, ProgramType};
use std::io::{Read, Seek};
use std::sync::Arc;
use thiserror::Error;

/// Represents a loaded SELF in an unmodified state (no code lifting, etc.).
pub struct Module<I: Read + Seek> {
    image: Elf<I>,
    memory: Memory,
}

impl<I: Read + Seek> Module<I> {
    pub fn load(mut image: Elf<I>, mm: Arc<MemoryManager>) -> Result<Self, LoadError> {
        // Map SELF to the memory.
        let mut memory = Memory::new(&image, mm)?;

        memory.load(|prog, buf| {
            if let Err(e) = image.read_program(prog, buf) {
                Err(LoadError::ReadProgramFailed(prog, e))
            } else {
                Ok(())
            }
        })?;

        memory.protect(&image)?;

        Ok(Self { image, memory })
    }

    pub fn image(&self) -> &Elf<I> {
        &self.image
    }

    pub fn memory(&self) -> &Memory {
        &self.memory
    }
}

/// Represents a memory of the module.
pub struct Memory {
    mm: Arc<MemoryManager>,
    ptr: *mut u8,
    len: usize,
    segments: Vec<MemorySegment>,
}

impl Memory {
    fn new<I: Read + Seek>(elf: &Elf<I>, mm: Arc<MemoryManager>) -> Result<Self, LoadError> {
        use crate::memory::{MappingFlags, Protections};

        let programs = elf.programs();

        // Create segments from programs.
        let mut segments: Vec<MemorySegment> = Vec::with_capacity(programs.len());

        for (i, p) in programs.iter().enumerate() {
            let t = p.ty();

            if t == ProgramType::PT_LOAD || t == ProgramType::PT_SCE_RELRO {
                let s = MemorySegment {
                    start: p.addr(),
                    len: p.aligned_size(),
                    program: i,
                };

                if s.len == 0 {
                    return Err(LoadError::ZeroLenProgram(i));
                }

                segments.push(s);
            }
        }

        if segments.is_empty() {
            return Err(LoadError::NoMappablePrograms);
        }

        // Make sure no any segment is overlapped.
        let mut len = 0;

        segments.sort_unstable_by_key(|s| s.start);

        for s in &segments {
            if s.start < len {
                return Err(LoadError::ProgramAddressOverlapped(s.program));
            }

            len += s.len;
        }

        // Allocate pages.
        let ptr = match mm.mmap(
            0,
            len,
            Protections::CPU_READ | Protections::CPU_WRITE,
            MappingFlags::MAP_ANON | MappingFlags::MAP_PRIVATE,
            -1,
            0,
        ) {
            Ok(v) => v,
            Err(e) => return Err(LoadError::MemoryAllocationFailed(len, e)),
        };

        Ok(Self {
            mm,
            ptr,
            len,
            segments,
        })
    }

    fn load<L, E>(&mut self, mut loader: L) -> Result<(), E>
    where
        L: FnMut(usize, &mut [u8]) -> Result<(), E>,
    {
        for seg in &self.segments {
            // Get destination buffer.
            let ptr = unsafe { self.ptr.add(seg.start) };
            let dst = unsafe { std::slice::from_raw_parts_mut(ptr, seg.len) };

            // Invoke loader.
            loader(seg.program, dst)?;
        }

        Ok(())
    }

    fn protect<I: Read + Seek>(&mut self, elf: &Elf<I>) -> Result<(), LoadError> {
        use crate::memory::Protections;

        let progs = elf.programs();

        for seg in &self.segments {
            // Derive protections from program flags.
            let flags = progs[seg.program].flags();
            let mut prot = Protections::NONE;

            if flags.contains(ProgramFlags::EXECUTE) {
                prot |= Protections::CPU_EXEC;
            }

            if flags.contains(ProgramFlags::READ) {
                prot |= Protections::CPU_READ;
            }

            if flags.contains(ProgramFlags::WRITE) {
                prot |= Protections::CPU_WRITE;
            }

            // Change protection.
            let addr = unsafe { self.ptr.add(seg.start) };

            if let Err(e) = self.mm.mprotect(addr, seg.len, prot) {
                return Err(LoadError::ChangeProtectionFailed(seg.program, e));
            }
        }

        Ok(())
    }

    pub fn addr(&self) -> usize {
        self.ptr as usize
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn segments(&self) -> &[MemorySegment] {
        self.segments.as_ref()
    }
}

impl AsRef<[u8]> for Memory {
    fn as_ref(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl Drop for Memory {
    fn drop(&mut self) {
        if let Err(e) = self.mm.munmap(self.ptr, self.len) {
            panic!(
                "Failed to unmap {} bytes starting at {:p}: {}.",
                self.len, self.ptr, e
            );
        }
    }
}

/// Contains information for a segment in [`Memory`].
pub struct MemorySegment {
    start: usize,
    len: usize,
    program: usize,
}

impl MemorySegment {
    /// Gets the offset within the module memory of this segment.
    pub fn start(&self) -> usize {
        self.start
    }

    pub fn len(&self) -> usize {
        self.len
    }

    /// Gets the corresponding index of SELF program.
    pub fn program(&self) -> usize {
        self.program
    }
}

/// Represents errors for [`Module::load()`].
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("program #{0} has zero size in the memory")]
    ZeroLenProgram(usize),

    #[error("no any mappable programs")]
    NoMappablePrograms,

    #[error("program #{0} has address overlapped with the other program")]
    ProgramAddressOverlapped(usize),

    #[error("cannot allocate {0} bytes")]
    MemoryAllocationFailed(usize, #[source] crate::memory::MmapError),

    #[error("cannot read program #{0}")]
    ReadProgramFailed(usize, #[source] elf::ReadProgramError),

    #[error("cannot change protection for mapped program #{0}")]
    ChangeProtectionFailed(usize, #[source] crate::memory::MprotectError),
}
