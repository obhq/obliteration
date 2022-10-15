use super::Segment;
use crate::process::Process;
use iced_x86::code_asm::{
    byte_ptr, get_gpr64, get_gpr8, qword_ptr, rax, rdi, rsi, CodeAssembler, CodeLabel,
};
use iced_x86::{BlockEncoderOptions, Code, Decoder, DecoderOptions, Instruction, OpKind, Register};
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::mem::transmute;

pub(super) struct Recompiler<'input> {
    proc: *mut Process,
    input: &'input [u8],
    segments: Vec<Segment>,
    assembler: CodeAssembler,
    jobs: VecDeque<(u64, usize, CodeLabel)>,
    labels: HashMap<u64, CodeLabel>, // Original address to recompiled label.
    output_size: usize, // Roughly estimation size of the output but not less than the actual size.
}

impl<'input> Recompiler<'input> {
    /// `input` is a mapped SELF.
    pub fn new(proc: *mut Process, input: &'input [u8], segments: Vec<Segment>) -> Self {
        Self {
            proc,
            input,
            segments,
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
                Code::Add_rm64_imm8 => self.transform_add_rm64_imm8(i),
                Code::Call_rm64 => self.transform_call_rm64(i),
                Code::Call_rel32_64 => self.transform_call_rel32(i),
                Code::Cmp_r64_rm64 => self.transform_cmp_r64_rm64(i),
                Code::Cmp_rm64_imm8 => self.transform_cmp_rm64_imm8(i),
                Code::Cmp_rm64_r64 => self.transform_cmp_rm64_r64(i),
                Code::Jae_rel8_64 => self.transform_jae_rel8_64(i),
                Code::Jb_rel8_64 => self.transform_jb_rel8_64(i),
                Code::Jbe_rel8_64 => self.transform_jbe_rel8_64(i),
                Code::Je_rel8_64 | Code::Je_rel32_64 => self.transform_je_rel(i),
                Code::Jmp_rm64 => {
                    end = true;
                    self.transform_jmp_rm64(i)
                }
                Code::Jmp_rel8_64 | Code::Jmp_rel32_64 => {
                    end = true;
                    self.transform_jmp_rel(i)
                }
                Code::Jne_rel8_64 | Code::Jne_rel32_64 => self.transform_jne_rel(i),
                Code::Lea_r64_m => self.transform_lea64(i),
                Code::Mov_r8_rm8 => self.transform_mov_r8_rm8(i),
                Code::Mov_r32_imm32 => self.preserve(i),
                Code::Mov_r32_rm32 => self.transform_mov_r32_rm32(i),
                Code::Mov_r64_rm64 => self.transform_mov_r64_rm64(i),
                Code::Mov_rm8_imm8 => self.transform_mov_rm8_imm8(i),
                Code::Mov_rm32_r32 => self.transform_mov_rm32_r32(i),
                Code::Mov_rm64_r64 => self.transform_mov_rm64_r64(i),
                Code::Nop_rm16 | Code::Nop_rm32 | Code::Nopw => self.preserve(i),
                Code::Pop_r64 => self.preserve(i),
                Code::Pushq_imm32 => self.preserve(i),
                Code::Push_r64 => self.preserve(i),
                Code::Retnq => {
                    end = true;
                    self.preserve(i)
                }
                Code::Sub_rm64_imm32 => self.transform_sub_rm64_imm32(i),
                Code::Test_rm8_r8 => self.transform_test_rm8_r8(i),
                Code::Test_rm32_r32 => self.transform_test_rm32_r32(i),
                Code::Test_rm64_r64 => self.transform_test_rm64_r64(i),
                Code::Ud2 => {
                    end = true;
                    self.transform_ud2(i)
                }
                Code::VEX_Vmovdqa_xmmm128_xmm => self.transform_vmovdqa_xmmm128_xmm(i),
                Code::VEX_Vmovq_xmm_rm64 => self.transform_vmovq_xmm_rm64(i),
                Code::VEX_Vpslldq_xmm_xmm_imm8 => self.preserve(i),
                Code::Xor_rm32_r32 => self.transform_xor_rm32_r32(i),
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

    fn transform_add_rm64_imm8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("ADD r/m64, imm8 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_call_rm64(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("CALL r/m64 with operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
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

    fn transform_cmp_r64_rm64(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let dst = i.op0_register();
            let src = i.ip_rel_memory_address();
            let tmp = get_gpr64(Self::temp_register64(dst)).unwrap();

            if self.is_executable(src) {
                panic!(
                    "CMP r64, r/m64 with second operand from executable segment is not supported."
                );
            }

            // Transform to absolute address.
            self.assembler.push(tmp).unwrap();
            self.assembler.mov(tmp, src).unwrap();
            self.assembler
                .cmp(get_gpr64(dst).unwrap(), qword_ptr(tmp))
                .unwrap();
            self.assembler.pop(tmp).unwrap();

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_cmp_rm64_imm8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("CMP r/m64, imm8 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_cmp_rm64_r64(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("CMP r/m64, r64 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_jae_rel8_64(&mut self, i: Instruction) -> usize {
        let dst = i.near_branch64();

        if let Some(&label) = self.labels.get(&dst) {
            self.assembler.jae(label).unwrap();
        } else {
            let label = self.assembler.create_label();

            self.assembler.jae(label).unwrap();
            self.jobs.push_back((dst, self.offset(dst), label));
        }

        15
    }

    fn transform_jb_rel8_64(&mut self, i: Instruction) -> usize {
        let dst = i.near_branch64();

        if let Some(&label) = self.labels.get(&dst) {
            self.assembler.jb(label).unwrap();
        } else {
            let label = self.assembler.create_label();

            self.assembler.jb(label).unwrap();
            self.jobs.push_back((dst, self.offset(dst), label));
        }

        15
    }

    fn transform_jbe_rel8_64(&mut self, i: Instruction) -> usize {
        let dst = i.near_branch64();

        if let Some(&label) = self.labels.get(&dst) {
            self.assembler.jbe(label).unwrap();
        } else {
            let label = self.assembler.create_label();

            self.assembler.jbe(label).unwrap();
            self.jobs.push_back((dst, self.offset(dst), label));
        }

        15
    }

    fn transform_je_rel(&mut self, i: Instruction) -> usize {
        let dst = i.near_branch64();

        if let Some(&label) = self.labels.get(&dst) {
            self.assembler.je(label).unwrap();
        } else {
            let label = self.assembler.create_label();

            self.assembler.je(label).unwrap();
            self.jobs.push_back((dst, self.offset(dst), label));
        }

        15
    }

    fn transform_jmp_rm64(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let dst = i.ip_rel_memory_address();

            if let Some(&label) = self.labels.get(&dst) {
                self.assembler.jmp(label).unwrap();
            } else {
                let label = self.assembler.create_label();

                self.assembler.jmp(label).unwrap();
                self.jobs.push_back((dst, self.offset(dst), label));
            }

            15
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_jmp_rel(&mut self, i: Instruction) -> usize {
        let dst = i.near_branch64();

        if let Some(&label) = self.labels.get(&dst) {
            self.assembler.jmp(label).unwrap();
        } else {
            let label = self.assembler.create_label();

            self.assembler.jmp(label).unwrap();
            self.jobs.push_back((dst, self.offset(dst), label));
        }

        15
    }

    fn transform_jne_rel(&mut self, i: Instruction) -> usize {
        let dst = i.near_branch64();

        if let Some(&label) = self.labels.get(&dst) {
            self.assembler.jne(label).unwrap();
        } else {
            let label = self.assembler.create_label();

            self.assembler.jne(label).unwrap();
            self.jobs.push_back((dst, self.offset(dst), label));
        }

        15
    }

    fn transform_lea64(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.is_ip_rel_memory_operand() {
            // Check if second operand already recompiled.
            let dst = get_gpr64(i.op0_register()).unwrap();
            let src = i.ip_rel_memory_address();

            if let Some(&label) = self.labels.get(&src) {
                self.assembler.lea(dst, qword_ptr(label)).unwrap();
            } else {
                // Check which segment the second operand fall under.
                let segment = self.segment(src);

                if segment.flags.is_executable() {
                    let label = self.assembler.create_label();

                    self.assembler.lea(dst, qword_ptr(label)).unwrap();
                    self.jobs.push_back((src, self.offset(src), label));
                } else {
                    self.assembler.mov(dst, src).unwrap();
                }
            }

            15
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_mov_r8_rm8(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let dst = i.op0_register();
            let src = i.ip_rel_memory_address();
            let dst64 = dst.full_register();
            let tmp = get_gpr64(Self::temp_register64(dst64)).unwrap();

            if self.is_executable(src) {
                panic!(
                    "MOV r8, r/m8 with second operand from executable segment is not supported."
                );
            }

            // Transform to absolute address.
            self.assembler.push(tmp).unwrap();
            self.assembler.mov(tmp, src).unwrap();
            self.assembler
                .mov(get_gpr8(dst).unwrap(), byte_ptr(tmp))
                .unwrap();
            self.assembler.pop(tmp).unwrap();

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_mov_r32_rm32(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("MOV r32, r/m32 with second operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_mov_r64_rm64(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let dst = i.op0_register();
            let src = i.ip_rel_memory_address();
            let tmp = get_gpr64(Self::temp_register64(dst)).unwrap();

            if self.is_executable(src) {
                panic!(
                    "MOV r64, r/m64 with second operand from executable segment is not supported."
                );
            }

            // Transform to absolute address.
            self.assembler.push(tmp).unwrap();
            self.assembler.mov(tmp, src).unwrap();
            self.assembler
                .mov(get_gpr64(dst).unwrap(), qword_ptr(tmp))
                .unwrap();
            self.assembler.pop(tmp).unwrap();

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_mov_rm8_imm8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let dst = i.ip_rel_memory_address();
            let src = i.immediate8();

            if self.is_executable(dst) {
                panic!(
                    "MOV r/m8, imm8 with first operand from executable segment is not supported."
                );
            }

            // Transform to absolute address.
            self.assembler.push(rax).unwrap();
            self.assembler.mov(rax, dst).unwrap();
            self.assembler.mov(byte_ptr(rax), src as u32).unwrap();
            self.assembler.pop(rax).unwrap();

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_mov_rm32_r32(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("MOV r/m32, r32 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_mov_rm64_r64(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("MOV r/m64, r64 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_sub_rm64_imm32(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("SUB r/m64, imm32 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_test_rm8_r8(&mut self, i: Instruction) -> usize {
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("TEST r/m8, r8 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_test_rm32_r32(&mut self, i: Instruction) -> usize {
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("TEST r/m32, r32 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_test_rm64_r64(&mut self, i: Instruction) -> usize {
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("TEST r/m64, r64 with first operand as RIP-relative is not supported yet.");
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

    fn transform_vmovdqa_xmmm128_xmm(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!(
                "VMOVDQA xmm2/m128, xmm1 with first operand as RIP-relative is not supported yet."
            );
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_vmovq_xmm_rm64(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("VMOVQ xmm1, r/m64 with second operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_xor_rm32_r32(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("XOR r/m32, r32 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn preserve(&mut self, i: Instruction) -> usize {
        self.assembler.add_instruction(i).unwrap();
        i.len()
    }

    /// Get register other than `keep`.
    fn temp_register64(keep: Register) -> Register {
        match keep {
            Register::RAX => Register::RBX,
            Register::RBX => Register::RAX,
            r => panic!("Register {:?} is not implemented yet.", r),
        }
    }

    fn offset(&self, addr: u64) -> usize {
        let base: u64 = unsafe { transmute(self.input.as_ptr()) };

        (addr - base) as usize
    }

    fn is_executable(&self, addr: u64) -> bool {
        self.segment(addr).flags.is_executable()
    }

    fn segment(&self, addr: u64) -> &Segment {
        let addr = addr as usize;

        for s in &self.segments {
            if addr >= s.start && addr < s.end {
                return s;
            }
        }

        panic!(
            "Address {:#018x} ({:#018x}) is not mapped.",
            addr,
            self.offset(addr as _)
        );
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
