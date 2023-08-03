use super::MapError;
use crate::memory::{MappingFlags, MemoryManager, MprotectError, Protections};
use elf::{Elf, ProgramFlags, ProgramType};
use std::alloc::Layout;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::marker::PhantomData;
use std::sync::{Mutex, MutexGuard};
use thiserror::Error;

/// A memory of the loaded module.
pub struct Memory {
    ptr: *mut u8,
    len: usize,
    base: usize,
    segments: Vec<MemorySegment>,
    code_index: usize,
    code_sealed: Mutex<usize>,
    data_index: usize,
    data_sealed: Mutex<usize>,
    destructors: Mutex<Vec<Box<dyn FnOnce()>>>,
}

impl Memory {
    pub(super) fn new(image: &Elf<File>, base: usize) -> Result<Self, MapError> {
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
            let mut prot = Protections::empty();

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
            // We need to check the PS4 kernel to see how it is handled this case.
            todo!("(S)ELF with no mappable segments is not supported yet.");
        }

        // Make sure no any segment is overlapped.
        let mut len = base;

        segments.sort_unstable_by_key(|s| s.start);

        for s in &segments {
            if s.start < len {
                // We need to check the PS4 kernel to see how it is handled this case.
                todo!("(S)ELF with overlapped programs is not supported yet.");
            }

            len = s.start + s.len;
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
        let mm = MemoryManager::current();
        let mut pages = match mm.mmap(
            0,
            len,
            Protections::empty(),
            MappingFlags::MAP_ANON | MappingFlags::MAP_PRIVATE,
            -1,
            0,
        ) {
            Ok(v) => v,
            Err(e) => return Err(MapError::MemoryAllocationFailed(len, e)),
        };

        // Apply memory protection.
        for seg in &segments {
            let addr = unsafe { pages.as_mut_ptr().add(seg.start) };
            let len = seg.len;
            let prot = seg.prot;

            if let Err(e) = mm.mprotect(addr, len, prot) {
                return Err(MapError::ProtectMemoryFailed(addr, len, prot, e));
            }
        }

        Ok(Self {
            ptr: pages.into_raw(),
            len,
            base,
            segments,
            code_index,
            code_sealed: Mutex::new(0),
            data_index,
            data_sealed: Mutex::new(0),
            destructors: Mutex::default(),
        })
    }

    pub fn addr(&self) -> usize {
        self.ptr as _
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn base(&self) -> usize {
        self.base
    }

    pub fn segments(&self) -> &[MemorySegment] {
        self.segments.as_ref()
    }

    /// # Safety
    /// Some part of the returned slice may not readable.
    pub unsafe fn as_bytes(&self) -> &[u8] {
        std::slice::from_raw_parts(self.ptr, self.len)
    }

    /// # Safety
    /// No other threads may execute the memory in the segment until the returned [`CodeWorkspace`]
    /// has been dropped.
    pub unsafe fn code_workspace(&self) -> Result<CodeWorkspace<'_>, CodeWorkspaceError> {
        let sealed = self.code_sealed.lock().unwrap();
        let seg = match self.unprotect_segment(self.code_index) {
            Ok(v) => v,
            Err(e) => {
                return Err(CodeWorkspaceError::UnprotectSegmentFailed(
                    self.code_index,
                    e,
                ));
            }
        };

        Ok(CodeWorkspace {
            ptr: unsafe { seg.ptr.add(*sealed) },
            len: seg.len - *sealed,
            seg,
            sealed,
        })
    }

    pub fn push_data<T: 'static>(&self, value: T) -> Option<*mut T> {
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
    pub unsafe fn unprotect_segment(
        &self,
        seg: usize,
    ) -> Result<UnprotectedSegment<'_>, UnprotectSegmentError> {
        let seg = &self.segments[seg];
        let ptr = self.ptr.add(seg.start);
        let len = seg.len;
        let prot = Protections::CPU_READ | Protections::CPU_WRITE;

        if let Err(e) = MemoryManager::current().mprotect(ptr, len, prot) {
            return Err(UnprotectSegmentError::MprotectFailed(ptr, len, prot, e));
        }

        Ok(UnprotectedSegment {
            ptr,
            len,
            prot: seg.prot,
            phantom: PhantomData,
        })
    }

    /// # Safety
    /// No other threads may access the memory until the returned [`UnprotectedMemory`] has been
    /// dropped.
    pub unsafe fn unprotect(&self) -> Result<UnprotectedMemory<'_>, UnprotectError> {
        // Get the end offset of non-custom segments.
        let mut end = 0;

        for s in &self.segments {
            // Check if segment is a custom segment.
            if s.program().is_none() {
                break;
            }

            // Update end offset.
            end = s.end();
        }

        // Unprotect the memory.
        let prot = Protections::CPU_READ | Protections::CPU_WRITE;

        if let Err(e) = MemoryManager::current().mprotect(self.ptr, end, prot) {
            return Err(UnprotectError::MprotectFailed(self.ptr, end, prot, e));
        }

        Ok(UnprotectedMemory {
            ptr: self.ptr,
            len: end,
            segments: &self.segments,
        })
    }
}

impl Drop for Memory {
    fn drop(&mut self) {
        // Run destructors.
        let destructors = self.destructors.get_mut().unwrap();

        for d in destructors.drain(..).rev() {
            d();
        }

        // Unmap the memory.
        MemoryManager::current().munmap(self.ptr, self.len).unwrap();
    }
}

impl Debug for Memory {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Memory")
            .field("ptr", &self.ptr)
            .field("len", &self.len)
            .field("base", &self.base)
            .field("segments", &self.segments)
            .field("code_index", &self.code_index)
            .field("code_sealed", &self.code_sealed)
            .field("data_index", &self.data_index)
            .field("data_sealed", &self.data_sealed)
            .field("destructors", &self.destructors.lock().unwrap().len())
            .finish()
    }
}

unsafe impl Send for Memory {}
unsafe impl Sync for Memory {}

/// A segment in the [`Memory`].
#[derive(Debug)]
pub struct MemorySegment {
    start: usize,
    len: usize,
    program: Option<usize>,
    prot: Protections,
}

impl MemorySegment {
    /// Gets the offset within the module memory of this segment.
    ///
    /// This offset already take base address into account.
    pub fn start(&self) -> usize {
        self.start
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn end(&self) -> usize {
        self.start + self.len
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
    ptr: *mut u8,
    len: usize,
    prot: Protections,
    phantom: PhantomData<&'a [u8]>,
}

impl<'a> AsMut<[u8]> for UnprotectedSegment<'a> {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<'a> Drop for UnprotectedSegment<'a> {
    fn drop(&mut self) {
        MemoryManager::current()
            .mprotect(self.ptr, self.len, self.prot)
            .unwrap();
    }
}

/// The unprotected form of [`Memory`], not including our custom segments.
pub struct UnprotectedMemory<'a> {
    ptr: *mut u8,
    len: usize,
    segments: &'a [MemorySegment],
}

impl<'a> Drop for UnprotectedMemory<'a> {
    fn drop(&mut self) {
        for s in self.segments {
            if s.program().is_none() {
                break;
            }

            let addr = unsafe { self.ptr.add(s.start()) };

            MemoryManager::current()
                .mprotect(addr, s.len(), s.prot())
                .unwrap();
        }
    }
}

impl<'a> AsRef<[u8]> for UnprotectedMemory<'a> {
    fn as_ref(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl<'a> AsMut<[u8]> for UnprotectedMemory<'a> {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
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

/// Represents an error when [`Memory::code_workspace()`] is failed.
#[derive(Debug, Error)]
pub enum CodeWorkspaceError {
    #[error("cannot unprotect segment {0}")]
    UnprotectSegmentFailed(usize, #[source] UnprotectSegmentError),
}

/// Represents an error when [`Memory::unprotect_segment()`] is failed.
#[derive(Debug, Error)]
pub enum UnprotectSegmentError {
    #[error("cannot protect {1:#018x} bytes starting at {0:p} with {2}")]
    MprotectFailed(*const u8, usize, Protections, #[source] MprotectError),
}

/// Represents an error when [`Memory::unprotect()`] is failed.
#[derive(Debug, Error)]
pub enum UnprotectError {
    #[error("cannot protect {1:#018x} bytes starting at {0:p} with {2}")]
    MprotectFailed(*const u8, usize, Protections, #[source] MprotectError),
}
