use super::{LoadError, ModuleWorkspace};
use crate::memory::{MemoryManager, MprotectError, Protections};
use elf::{Elf, ProgramFlags, ProgramType};
use std::fs::File;
use std::ops::{Index, IndexMut};
use std::slice::SliceIndex;

/// Represents a memory of the module.
pub struct Memory<'a> {
    mm: &'a MemoryManager,
    ptr: *mut u8,
    len: usize,
    segments: Vec<MemorySegment>,
    workspace: ModuleWorkspace<'a>,
}

impl<'a> Memory<'a> {
    pub(super) fn new(
        elf: &Elf<File>,
        mm: &'a MemoryManager,
        workspace: usize,
    ) -> Result<Self, LoadError> {
        use crate::memory::MappingFlags;

        let programs = elf.programs();

        // Create segments from programs.
        let mut segments: Vec<MemorySegment> = Vec::with_capacity(programs.len());

        for (i, p) in programs.iter().enumerate() {
            let t = p.ty();

            if t == ProgramType::PT_LOAD || t == ProgramType::PT_SCE_RELRO {
                // Check if size in memory valid.
                let len = p.aligned_size();

                if len == 0 {
                    return Err(LoadError::ZeroLenProgram(i));
                }

                // Get protection.
                let flags = p.flags();
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

                // Construct the segment info.
                segments.push(MemorySegment {
                    start: p.addr(),
                    len,
                    program: i,
                    prot,
                });
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
            len + workspace,
            Protections::CPU_READ | Protections::CPU_WRITE | Protections::CPU_EXEC,
            MappingFlags::MAP_ANON | MappingFlags::MAP_PRIVATE,
            -1,
            0,
        ) {
            Ok(v) => v,
            Err(e) => return Err(LoadError::MemoryAllocationFailed(len + workspace, e)),
        };

        Ok(Self {
            mm,
            ptr,
            len,
            workspace: ModuleWorkspace::new(mm, unsafe { ptr.add(len) }, workspace),
            segments,
        })
    }

    pub(super) fn load<L, E>(&mut self, mut loader: L) -> Result<(), E>
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

    pub fn addr(&self) -> usize {
        self.ptr as usize
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn segments(&self) -> &[MemorySegment] {
        self.segments.as_ref()
    }

    pub fn workspace(&self) -> &ModuleWorkspace {
        &self.workspace
    }

    pub(super) fn protect(&self) -> Result<(), MprotectError> {
        for seg in &self.segments {
            let addr = unsafe { self.ptr.add(seg.start) };

            self.mm.mprotect(addr, seg.len, seg.prot)?;
        }

        Ok(())
    }

    /// # Safety
    /// Only a single thread can have access to the unprotected memory.
    pub unsafe fn unprotect(&self) -> Result<UnprotectedMemory<'_>, MprotectError> {
        self.mm.mprotect(
            self.ptr,
            self.len,
            Protections::CPU_READ | Protections::CPU_WRITE,
        )?;

        Ok(UnprotectedMemory(self))
    }
}

impl<'a> AsRef<[u8]> for Memory<'a> {
    fn as_ref(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<'a> Drop for Memory<'a> {
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
    prot: Protections,
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

    pub fn prot(&self) -> Protections {
        self.prot
    }
}

/// Represents the memory of the module in unprotected form.
pub struct UnprotectedMemory<'a>(&'a Memory<'a>);

impl<'a> UnprotectedMemory<'a> {
    pub fn addr(&self) -> usize {
        self.0.addr()
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.0.ptr, self.0.len) }
    }
}

impl<'a> Drop for UnprotectedMemory<'a> {
    fn drop(&mut self) {
        if let Err(e) = self.0.protect() {
            // This should never happen because it was succeeded when the memory is initialized.
            panic!("Cannot protect memory: {e}.");
        }
    }
}

impl<'a, I> Index<I> for UnprotectedMemory<'a>
where
    I: SliceIndex<[u8]>,
{
    type Output = <I as SliceIndex<[u8]>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<'a, I> IndexMut<I> for UnprotectedMemory<'a>
where
    I: SliceIndex<[u8]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.as_mut_slice()[index]
    }
}
