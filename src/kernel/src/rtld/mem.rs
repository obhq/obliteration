use super::LoadError;
use crate::memory::{MappingFlags, MemoryManager, MprotectError, Protections};
use elf::{Elf, ProgramFlags, ProgramType};
use std::alloc::Layout;
use std::fs::File;
use std::sync::{Mutex, MutexGuard};

/// A memory of the loaded module.
pub struct Memory<'a> {
    mm: &'a MemoryManager,
    ptr: *mut u8,
    len: usize,
    segments: Vec<MemorySegment>,
    code_index: usize,
    code_sealed: Mutex<usize>,
    data_index: usize,
    data_sealed: Mutex<usize>,
    destructors: Mutex<Vec<Box<dyn FnOnce() + 'a>>>,
}

impl<'a> Memory<'a> {
    pub(super) fn new(
        mm: &'a MemoryManager,
        image: &Elf<File>,
        base: usize,
    ) -> Result<Self, LoadError> {
        // Create segments from ELF programs.
        let programs = image.programs();
        let mut segments: Vec<MemorySegment> = Vec::with_capacity(programs.len() + 2);

        for (i, prog) in programs.iter().enumerate() {
            // Skip if unmappable program.
            let ty = prog.ty();

            if ty != ProgramType::PT_LOAD && ty != ProgramType::PT_SCE_RELRO {
                continue;
            }

            // Skip if memory size is zero.
            if prog.memory_size() == 0 {
                continue;
            }

            // Get protection.
            let flags = prog.flags();
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
                start: prog.addr() + base,
                len: prog.aligned_size(),
                program: Some(i),
                prot,
            });
        }

        if segments.is_empty() {
            todo!("(S)ELF with no mappable segments is not supported yet.");
        }

        // Make sure no any segment is overlapped.
        let mut len = base;

        segments.sort_unstable_by_key(|s| s.start);

        for s in &segments {
            if s.start < len {
                todo!("(S)ELF with overlapped programs is not supported yet.");
            }

            len += s.len;
        }

        // Create workspace for code.
        let code_index = segments.len();
        let segment = MemorySegment {
            start: len,
            len: 1024 * 1024,
            program: None,
            prot: Protections::CPU_READ | Protections::CPU_EXEC,
        };

        len += segment.len;
        segments.push(segment);

        // Create workspace for data. We cannot mix this the code because the executable-space
        // protection on some system don't allow execution on writable page.
        let data_index = segments.len();
        let segment = MemorySegment {
            start: len,
            len: 1024 * 1024,
            program: None,
            prot: Protections::CPU_READ | Protections::CPU_WRITE,
        };

        len += segment.len;
        segments.push(segment);

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
            code_index,
            code_sealed: Mutex::new(0),
            data_index,
            data_sealed: Mutex::new(0),
            destructors: Mutex::default(),
        })
    }

    pub(super) fn load<L, E>(&mut self, mut loader: L) -> Result<(), E>
    where
        L: FnMut(usize, &mut [u8]) -> Result<(), E>,
    {
        for seg in &self.segments {
            // Get target program.
            let prog = match seg.program {
                Some(v) => v,
                None => continue,
            };

            // Get destination buffer.
            let ptr = unsafe { self.ptr.add(seg.start) };
            let dst = unsafe { std::slice::from_raw_parts_mut(ptr, seg.len) };

            // Invoke loader.
            loader(prog, dst)?;
        }

        Ok(())
    }

    pub(super) fn protect(&mut self) -> Result<(), MprotectError> {
        for seg in &self.segments {
            let addr = unsafe { self.ptr.add(seg.start) };
            self.mm.mprotect(addr, seg.len, seg.prot)?;
        }

        Ok(())
    }

    pub fn addr(&self) -> usize {
        self.ptr as _
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn segments(&self) -> &[MemorySegment] {
        self.segments.as_ref()
    }

    /// # Safety
    /// No other threads may execute the memory in the segment until the returned [`CodeWorkspace`]
    /// has been dropped.
    pub unsafe fn code_workspace(&self) -> Result<CodeWorkspace<'_>, MprotectError> {
        let sealed = self.code_sealed.lock().unwrap();
        let seg = self.unprotect(self.code_index)?;

        Ok(CodeWorkspace {
            ptr: unsafe { seg.ptr.add(*sealed) },
            len: seg.len - *sealed,
            seg,
            sealed,
        })
    }

    pub fn push_data<T: 'a>(&self, value: T) -> Option<*mut T> {
        let mut sealed = self.data_sealed.lock().unwrap();
        let seg = &self.segments[self.data_index];
        let ptr = unsafe { self.ptr.add(seg.start + *sealed) };
        let available = seg.len - *sealed;

        // Check if the remaining space is enough.
        let layout = Layout::new::<T>();
        let offset = match (ptr as usize) % layout.align() {
            0 => 0,
            v => layout.align() - v,
        };

        if offset + layout.size() > available {
            return None;
        }

        // Move value to the workspace.
        let ptr = unsafe { ptr.add(offset) } as *mut T;

        unsafe { std::ptr::write(ptr, value) };

        self.destructors
            .lock()
            .unwrap()
            .push(Box::new(move || unsafe { std::ptr::drop_in_place(ptr) }));

        // Seal the memory.
        *sealed += offset + layout.size();

        Some(ptr)
    }

    /// # Safety
    /// No other threads may access the memory in the segment until the returned
    /// [`UnprotectedSegment`] has been dropped.
    ///
    /// # Panics
    /// `seg` is not a valid segment.
    pub unsafe fn unprotect(&self, seg: usize) -> Result<UnprotectedSegment<'_>, MprotectError> {
        let seg = &self.segments[seg];
        let ptr = self.ptr.add(seg.start);
        let len = seg.len;

        self.mm
            .mprotect(ptr, len, Protections::CPU_READ | Protections::CPU_WRITE)?;

        Ok(UnprotectedSegment {
            mm: self.mm,
            ptr,
            len,
            prot: seg.prot,
        })
    }
}

impl<'a> AsRef<[u8]> for Memory<'a> {
    fn as_ref(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<'a> Drop for Memory<'a> {
    fn drop(&mut self) {
        // Run destructors.
        let destructors = self.destructors.get_mut().unwrap();

        for d in destructors.drain(..).rev() {
            d();
        }

        // Unmap the memory.
        if let Err(e) = self.mm.munmap(self.ptr, self.len) {
            panic!(
                "Failed to unmap {} bytes starting at {:p}: {}.",
                self.len, self.ptr, e
            );
        }
    }
}

/// A segment in the [`Memory`].
pub struct MemorySegment {
    start: usize,
    len: usize,
    program: Option<usize>,
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

    /// Gets the corresponding index of (S)ELF program.
    pub fn program(&self) -> Option<usize> {
        self.program
    }

    pub fn prot(&self) -> Protections {
        self.prot
    }
}

/// A memory segment in an unprotected form.
pub struct UnprotectedSegment<'a> {
    mm: &'a MemoryManager,
    ptr: *mut u8,
    len: usize,
    prot: Protections,
}

impl<'a> AsMut<[u8]> for UnprotectedSegment<'a> {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<'a> Drop for UnprotectedSegment<'a> {
    fn drop(&mut self) {
        if let Err(e) = self.mm.mprotect(self.ptr, self.len, self.prot) {
            panic!("Cannot protect memory: {e}.");
        }
    }
}

/// An exclusive access to the unsealed code workspace.
pub struct CodeWorkspace<'a> {
    ptr: *mut u8,
    len: usize,
    seg: UnprotectedSegment<'a>,
    sealed: MutexGuard<'a, usize>,
}

impl<'a> CodeWorkspace<'a> {
    pub fn addr(&self) -> usize {
        self.ptr as _
    }

    pub fn seal(mut self, len: usize) {
        if len > self.len {
            panic!("The amount to seal is larger than available space.");
        }

        *self.sealed += len;

        drop(self.seg);
    }
}

impl<'a> AsMut<[u8]> for CodeWorkspace<'a> {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}
