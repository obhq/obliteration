use super::Process;
use iced_x86::code_asm::{get_gpr64, qword_ptr, rdi, rsi, CodeAssembler, CodeLabel};
use iced_x86::{BlockEncoderOptions, Code, Decoder, DecoderOptions, Instruction};
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::mem::transmute;

pub(super) struct Recompiler<'input> {
    input: &'input [u8],
    proc: *mut Process,
    assembler: CodeAssembler,
    jobs: VecDeque<(u64, usize, CodeLabel)>,
    labels: HashMap<u64, CodeLabel>, // Original address to recompiled label.
    output_size: usize, // Roughly estimation size of the output but not less than the actual size.
}

impl<'input> Recompiler<'input> {
    /// `input` is a mapped SELF.
    pub fn new(input: &'input [u8], proc: *mut Process) -> Self {
        Self {
            input,
            proc,
            assembler: CodeAssembler::new(64).unwrap(),
            jobs: VecDeque::new(),
            labels: HashMap::new(),
            output_size: 0,
        }
    }

    /// All items in `starts` **MUST** be unique and the `input` that was specified in [`new`]
    /// **MUST** outlive the returned [`NativeCode`].
    pub fn run(mut self, starts: &[usize]) -> Result<(NativeCode, Vec<*const u8>), RunError> {
        // Recompile all start offset.
        let mut start_addrs: Vec<u64> = Vec::with_capacity(starts.len());

        for &start in starts {
            // Recompile start offset.
            let label = self.assembler.create_label();
            let addr = self.recompile(start, label)?;

            start_addrs.push(addr);

            // Recompile all of references recursively.
            while !self.jobs.is_empty() {
                let (addr, offset, label) = unsafe { self.jobs.pop_front().unwrap_unchecked() };

                // Skip job for the same destination as previous job. The example cases for this
                // scenario is the block contains multiple jmp to the same location.
                if self.labels.contains_key(&addr) {
                    continue;
                }

                self.recompile(offset, label)?;
            }
        }

        // Allocate executable pages.
        let size = self.aligned_output_size();
        let mut native = match NativeCode::new(size) {
            Ok(v) => v,
            Err(e) => return Err(RunError::AllocatePagesFailed(size, e)),
        };

        // Assemble.
        let assembled = match self.assembler.assemble_options(
            native.addr() as _,
            BlockEncoderOptions::RETURN_NEW_INSTRUCTION_OFFSETS,
        ) {
            Ok(v) => v,
            Err(e) => return Err(RunError::AssembleFailed(e)),
        };

        // TODO: Remove writability from pages.
        // Copy assembled to executable page.
        native.copy_from(assembled.inner.code_buffer.as_slice());

        // Get entry address.
        let mut start_ptrs: Vec<*const u8> = Vec::with_capacity(start_addrs.len());

        for addr in start_addrs {
            let label = self.labels.get(&addr).unwrap();
            let addr = assembled.label_ip(label).unwrap();

            start_ptrs.push(unsafe { transmute(addr) });
        }

        Ok((native, start_ptrs))
    }

    fn recompile(&mut self, offset: usize, label: CodeLabel) -> Result<u64, RunError> {
        // Setup decoder.
        let input = self.input;
        let base: u64 = unsafe { transmute(input.as_ptr()) };
        let decoder = Decoder::with_ip(
            64,
            &input[offset..],
            base + offset as u64,
            DecoderOptions::AMD,
        );

        // Re-assemble offset until return.
        let start = offset;

        for i in decoder {
            // Check if instruction valid.
            let offset = (i.ip() - base) as usize;

            if i.is_invalid() {
                return Err(RunError::InvalidInstruction(offset));
            }

            // Map address to the label.
            match self.labels.entry(i.ip()) {
                std::collections::hash_map::Entry::Occupied(v) => {
                    self.assembler.jmp(v.get().clone()).unwrap();
                    self.output_size += 15;
                    break;
                }
                std::collections::hash_map::Entry::Vacant(v) => {
                    let label = if offset == start {
                        label
                    } else {
                        self.assembler.create_label()
                    };

                    self.assembler.set_label(v.insert(label)).unwrap();
                }
            }

            // Transform instruction.
            let mut end = false;

            self.output_size += match i.code() {
                Code::Call_rel32_64 => self.transform_call_rel32(i),
                Code::Lea_r64_m => self.transform_lea64(i),
                Code::Mov_r32_rm32 => self.preserve(i),
                Code::Mov_rm32_r32 => self.preserve(i),
                Code::Mov_rm64_r64 => self.preserve(i),
                Code::Push_r64 => self.preserve(i),
                Code::Ud2 => {
                    end = true;
                    self.transform_ud2(i)
                }
                Code::Xor_rm32_r32 => self.preserve(i),
                _ => {
                    return Err(RunError::UnknownInstruction(
                        offset,
                        (&input[offset..(offset + i.len())]).into(),
                        i,
                    ))
                }
            };

            if end {
                break;
            }
        }

        Ok(base + start as u64)
    }

    fn transform_call_rel32(&mut self, i: Instruction) -> usize {
        let dest = i.near_branch64();

        if let Some(label) = self.labels.get(&dest) {
            self.assembler.call(label.clone()).unwrap();
        } else {
            let label = self.assembler.create_label();

            self.assembler.call(label).unwrap();
            self.jobs.push_back((dest, self.offset(dest), label));
        }

        15
    }

    fn transform_lea64(&mut self, i: Instruction) -> usize {
        if i.is_ip_rel_memory_operand() {
            // TODO: Check if source address fall under data segment.
            let dst = get_gpr64(i.op0_register()).unwrap();
            let src = i.ip_rel_memory_address();

            if let Some(&label) = self.labels.get(&src) {
                self.assembler.lea(dst, qword_ptr(label)).unwrap();
            } else {
                let label = self.assembler.create_label();

                self.assembler.lea(dst, qword_ptr(label)).unwrap();
                self.jobs.push_back((src, self.offset(src), label));
            }

            15
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_ud2(&mut self, i: Instruction) -> usize {
        let handler: extern "sysv64" fn(&mut Process, usize) -> ! = Process::handle_ud2;
        let handler: u64 = unsafe { transmute(handler) };
        let proc: u64 = unsafe { transmute(self.proc) };

        self.assembler.mov(rsi, self.offset(i.ip()) as u64).unwrap();
        self.assembler.mov(rdi, proc).unwrap();
        self.assembler.call(handler).unwrap();

        15 * 3
    }

    fn preserve(&mut self, i: Instruction) -> usize {
        self.assembler.add_instruction(i).unwrap();
        i.len()
    }

    fn offset(&self, addr: u64) -> usize {
        (addr as usize) - unsafe { transmute::<*const u8, usize>(self.input.as_ptr()) }
    }

    fn aligned_output_size(&self) -> usize {
        let page_size = Self::page_size();
        let mut page_count = self.output_size / page_size;

        if page_count == 0 || self.output_size % page_size != 0 {
            page_count += 1;
        }

        page_count * page_size
    }

    #[cfg(unix)]
    fn page_size() -> usize {
        let v = unsafe { libc::sysconf(libc::_SC_PAGE_SIZE) };

        if v < 0 {
            // This should never happen.
            let e = std::io::Error::last_os_error();
            panic!("Failed to get page size: {}", e);
        }

        v as _
    }

    #[cfg(windows)]
    fn page_size() -> usize {
        use windows_sys::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
        let mut i: SYSTEM_INFO = util::mem::uninit();

        unsafe { GetSystemInfo(&mut i) };

        i.dwPageSize as _
    }
}

pub struct NativeCode {
    ptr: *mut u8,
    len: usize,
}

impl NativeCode {
    #[cfg(unix)]
    fn new(len: usize) -> Result<Self, std::io::Error> {
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_EXEC | libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANON,
                -1,
                0,
            )
        };

        if ptr == libc::MAP_FAILED {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(Self { ptr: ptr as _, len })
        }
    }

    #[cfg(windows)]
    fn new(len: usize) -> Result<Self, std::io::Error> {
        use std::ptr::null;
        use windows_sys::Win32::System::Memory::{
            VirtualAlloc, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE,
        };

        let ptr = unsafe {
            VirtualAlloc(
                null(),
                len,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_EXECUTE_READWRITE,
            )
        };

        if ptr.is_null() {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(Self { ptr: ptr as _, len })
        }
    }

    pub fn addr(&self) -> usize {
        unsafe { transmute(self.ptr) }
    }

    fn copy_from(&mut self, src: &[u8]) {
        debug_assert!(src.len() <= self.len);
        unsafe { self.ptr.copy_from_nonoverlapping(src.as_ptr(), src.len()) };
    }
}

impl Drop for NativeCode {
    #[cfg(unix)]
    fn drop(&mut self) {
        if unsafe { libc::munmap(self.ptr as _, self.len) } < 0 {
            let e = std::io::Error::last_os_error();

            panic!(
                "Failed to unmap {} bytes starting at {:p}: {}",
                self.len, self.ptr, e
            );
        }
    }

    #[cfg(windows)]
    fn drop(&mut self) {
        use windows_sys::Win32::System::Memory::{VirtualFree, MEM_RELEASE};

        if unsafe { VirtualFree(self.ptr as _, 0, MEM_RELEASE) } == 0 {
            let e = std::io::Error::last_os_error();

            panic!(
                "Failed to free {} bytes starting at {:p}: {}",
                self.len, self.ptr, e
            );
        }
    }
}

#[derive(Debug)]
pub enum RunError {
    InvalidInstruction(usize),
    UnknownInstruction(usize, Vec<u8>, Instruction),
    AllocatePagesFailed(usize, std::io::Error),
    AssembleFailed(iced_x86::IcedError),
}

impl Error for RunError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::AllocatePagesFailed(_, e) => Some(e),
            Self::AssembleFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for RunError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidInstruction(o) => {
                write!(f, "invalid instruction at {:#018x}", o)
            }
            Self::UnknownInstruction(o, r, i) => {
                write!(f, "unknown instruction '{}' ({:02x?}) at {:#018x}", i, r, o)
            }
            Self::AllocatePagesFailed(s, _) => write!(f, "cannot allocate pages for {} bytes", s),
            Self::AssembleFailed(_) => f.write_str("cannot assemble"),
        }
    }
}
