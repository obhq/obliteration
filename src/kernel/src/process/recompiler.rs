use super::Process;
use iced_x86::code_asm::{rdi, rsi, CodeAssembler, CodeLabel};
use iced_x86::{BlockEncoderOptions, Code, Decoder, DecoderOptions, Instruction};
use std::collections::VecDeque;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::mem::transmute;

pub(super) struct Recompiler<'input> {
    input: &'input [u8],
    proc: *mut Process,
    assembler: CodeAssembler,
    jobs: VecDeque<(usize, CodeLabel)>,
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
            output_size: 0,
        }
    }

    /// The `input` that was specified in [`new`] **MUST** outlive the returned [`NativeCode`].
    pub fn run(mut self, starts: &[usize]) -> Result<(NativeCode, Vec<*const u8>), RunError> {
        // Recompile all start offset.
        let mut start_labels: Vec<CodeLabel> = Vec::new();

        for &start in starts {
            let mut label = self.assembler.create_label();

            self.assembler.set_label(&mut label).unwrap();
            self.recompile(start)?;

            start_labels.push(label);
        }

        // Recompile all of references recursively.
        while !self.jobs.is_empty() {
            let (offset, mut label) = unsafe { self.jobs.pop_front().unwrap_unchecked() };

            self.assembler.set_label(&mut label).unwrap();
            self.recompile(offset)?;
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
        let mut start_addrs: Vec<*const u8> = Vec::with_capacity(start_labels.len());

        for label in start_labels {
            let addr = assembled.label_ip(&label).unwrap();
            start_addrs.push(unsafe { transmute(addr) });
        }

        Ok((native, start_addrs))
    }

    fn recompile(&mut self, offset: usize) -> Result<(), RunError> {
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
        for i in decoder {
            // Check if instruction valid.
            let offset = (i.ip() - base) as usize;

            if i.is_invalid() {
                return Err(RunError::InvalidInstruction(offset));
            }

            // Transform instruction.
            let mut end = false;

            self.output_size += match i.code() {
                Code::Call_rel32_64 => self.transform_call_rel32(i),
                Code::Lea_r64_m => self.preserve(i),
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

        Ok(())
    }

    fn transform_call_rel32(&mut self, i: Instruction) -> usize {
        let label = self.assembler.create_label();
        let offset = self.offset(i.near_branch64());

        self.assembler.call(label).unwrap();
        self.jobs.push_back((offset, label));

        15
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

    pub fn len(&self) -> usize {
        self.len
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
