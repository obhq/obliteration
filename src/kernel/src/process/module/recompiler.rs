use super::Segment;
use crate::process::Process;
use iced_x86::code_asm::{
    get_gpr64, get_gpr32, get_gpr8, dword_ptr, qword_ptr, byte_ptr, rax, rdi, rsi, CodeAssembler, CodeLabel,
};
use iced_x86::{BlockEncoderOptions, Code, Decoder, DecoderOptions, Instruction, OpKind, Register};
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::mem::transmute;

use std::env::consts::ARCH;

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
    #[cfg(target_arch = "x86_64")]
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

    #[cfg(not(target_arch = "x86_64"))]
    pub fn run(mut self, starts: &[usize]) -> Result<(NativeCode, Vec<*const u8>), RunError> {
        Err(RunError::UnsupportedArchError)
    }

    #[cfg(target_arch = "x86_64")]
    fn recompile(&mut self, offset: usize, label: CodeLabel) -> Result<u64, RunError> {
        // Setup decoder.
        let input = self.input;
        let base: u64 = input.as_ptr() as u64;
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
                    self.assembler.jmp(*v.get()).unwrap();
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
            let (size, end) = match i.code() {
                Code::Add_r32_rm32 => (self.transform_add_r32_rm32(i), false),
                Code::Add_r64_rm64 => (self.transform_add_r64_rm64(i), false),
                Code::Add_rm8_imm8 => (self.transform_add_rm8_imm8(i), false),
                Code::Add_rm8_r8 => (self.transform_add_rm8_r8(i), false),
                Code::Add_rm32_r32 => (self.transform_add_rm32_r32(i), false),
                Code::Add_rm64_imm8 => (self.transform_add_rm64_imm8(i), false),
                Code::Add_rm64_imm32 => (self.transform_add_rm64_imm32(i), false),
                Code::Add_rm64_r64 => (self.transform_add_rm64_r64(i), false),
                Code::And_EAX_imm32 => (self.preserve(i), false),
                Code::And_rm8_imm8 => (self.transform_and_rm8_imm8(i), false),
                Code::And_rm32_imm8 => (self.transform_and_rm32_imm8(i), false),
                Code::And_rm32_imm32 => (self.transform_and_rm32_imm32(i), false),
                Code::And_rm64_imm8 => (self.transform_and_rm64_imm8(i), false),
                Code::And_rm64_imm32 => (self.transform_and_rm64_imm32(i), false),
                Code::Call_rm64 => (self.transform_call_rm64(i), false),
                Code::Call_rel32_64 => (self.transform_call_rel32(i), false),
                Code::Cmove_r64_rm64 => (self.transform_cmove_r64_rm64(i), false),
                Code::Cmovne_r64_rm64 => (self.transform_cmovne_r64_rm64(i), false),
                Code::Cmp_RAX_imm32 => (self.preserve(i), false),
                Code::Cmp_r8_rm8 => (self.transform_cmp_r8_rm8(i), false),
                Code::Cmp_rm8_imm8 => (self.transform_cmp_rm8_imm8(i), false),
                Code::Cmp_rm8_r8 => (self.transform_cmp_rm8_r8(i), false),
                Code::Cmp_rm32_imm8 => (self.transform_cmp_rm32_imm8(i), false),
                Code::Cmp_rm32_r32 => (self.transform_cmp_rm32_r32(i), false),
                Code::Cmp_rm64_imm8 => (self.transform_cmp_rm64_imm8(i), false),
                Code::Cmp_r64_rm64 => (self.transform_cmp_r64_rm64(i), false),
                Code::Cmp_rm64_r64 => (self.transform_cmp_rm64_r64(i), false),
                Code::Cmpsb_m8_m8 => (self.transform_cmpsb_m8_m8(i), false),
                Code::Dec_rm32 => (self.transform_dec_rm32(i), false),
                Code::Dec_rm64 => (self.transform_dec_rm64(i), false),
                Code::Imul_r32_rm32_imm8 => (self.transform_imul_r32_rm32_imm8(i), false),
                Code::Imul_r32_rm32_imm32 => (self.transform_imul_r32_rm32_imm32(i), false),
                Code::In_AL_imm8 | Code::In_EAX_imm8 => (self.preserve(i), false),
                Code::Inc_rm32 => (self.transform_inc_rm32(i), false),
                Code::Inc_rm64 => (self.transform_inc_rm64(i), false),
                Code::Ja_rel8_64 | Code::Ja_rel32_64 => (self.transform_ja_rel(i), false),
                Code::Jae_rel8_64 | Code::Jae_rel32_64 => (self.transform_jae_rel(i), false),
                Code::Jb_rel8_64 | Code::Jb_rel32_64 => (self.transform_jb_rel(i), false),
                Code::Jbe_rel8_64 | Code::Jbe_rel32_64 => (self.transform_jbe_rel(i), false),
                Code::Je_rel8_64 | Code::Je_rel32_64 => (self.transform_je_rel(i), false),
                Code::Jg_rel8_64 | Code::Jg_rel32_64 => (self.transform_jg_rel(i), false),
                Code::Jge_rel8_64 | Code::Jge_rel32_64 => (self.transform_jge_rel(i), false),
                Code::Jl_rel8_64 | Code::Jl_rel32_64 => (self.transform_jl_rel(i), false),
                Code::Jle_rel8_64 | Code::Jle_rel32_64 => (self.transform_jle_rel(i), false),
                Code::Jmp_rel8_64 | Code::Jmp_rel32_64 => (self.transform_jmp_rel(i), true),
                Code::Jmp_rm64 => (self.transform_jmp_rm64(i), true),
                Code::Jne_rel8_64 | Code::Jne_rel32_64 => (self.transform_jne_rel(i), false),
                Code::Jns_rel8_64 | Code::Jns_rel32_64 => (self.transform_jns_rel(i), false),
                Code::Jo_rel8_64 | Code::Jo_rel32_64 => (self.transform_jo_rel(i), false),
                Code::Js_rel8_64 | Code::Js_rel32_64 => (self.transform_js_rel(i), false),
                Code::Lea_r32_m => (self.transform_lea32(i), false),
                Code::Lea_r64_m => (self.transform_lea64(i), false),
                Code::Mov_r8_imm8 => (self.preserve(i), false),
                Code::Mov_r8_rm8 => (self.transform_mov_r8_rm8(i), false),
                Code::Mov_r32_imm32 => (self.preserve(i), false),
                Code::Mov_r32_rm32 => (self.transform_mov_r32_rm32(i), false),
                Code::Mov_r64_rm64 => (self.transform_mov_r64_rm64(i), false),
                Code::Mov_rm8_imm8 => (self.transform_mov_rm8_imm8(i), false),
                Code::Mov_rm32_imm32 => (self.transform_mov_rm32_imm32(i), false),
                Code::Mov_rm64_imm32 => (self.transform_mov_rm64_imm32(i), false),
                Code::Mov_rm8_r8 => (self.transform_mov_rm8_r8(i), false),
                Code::Mov_rm32_r32 => (self.transform_mov_rm32_r32(i), false),
                Code::Mov_rm64_r64 => (self.transform_mov_rm64_r64(i), false),
                Code::Movaps_xmm_xmmm128 => (self.transform_movaps_xmm_xmmm128(i), false),
                Code::Movd_xmm_rm32 => (self.transform_movd_xmm_rm32(i), false),
                Code::Movdqu_xmmm128_xmm => (self.transform_movdqu_xmmm128_xmm(i), false),
                Code::Movsx_r32_rm8 => (self.transform_movsx_r32_rm8(i), false),
                Code::Movsxd_r32_rm32 => (self.transform_movsxd_r32_rm32(i), false),
                Code::Movsxd_r64_rm32 => (self.transform_movsxd_r64_rm32(i), false),
                Code::Movzx_r32_rm8 => (self.transform_movzx_r32_rm8(i), false),
                Code::Neg_rm32 => (self.transform_neg_rm32(i), false),
                Code::Neg_rm64 => (self.transform_neg_rm64(i), false),
                Code::Or_rm64_imm8 => (self.transform_or_rm64_imm8(i), false),
                Code::Or_rm64_r64 => (self.transform_or_rm64_r64(i), false),
                Code::Out_imm8_AL | Code::Out_imm8_EAX => (self.preserve(i), false),
                Code::Outsb_DX_m8 => (self.transform_outsb_dx_m8(i), false),
                Code::Nop_rm16 | Code::Nop_rm32 | Code::Nopd | Code::Nopw => (self.preserve(i), false),
                Code::Pop_r64 => (self.preserve(i), false),
                Code::Pshufd_xmm_xmmm128_imm8 => (self.transform_pshufd_xmm_xmmm128_imm8(i), false),
                Code::Pushq_imm32 => (self.preserve(i), false),
                Code::Push_r64 => (self.preserve(i), false),
                Code::Push_rm32 => (self.transform_push_rm32(i), false),
                Code::Push_rm64 => (self.transform_push_rm64(i), false),
                Code::Retnq => (self.preserve(i), true),
                Code::Sar_rm64_imm8 => (self.transform_sar_rm64_imm8(i), false),
                Code::Sbb_rm32_imm8 => (self.transform_sbb_rm32_imm8(i), false),
                Code::Sete_rm8 => (self.transform_sete_rm8(i), false),
                Code::Setne_rm8 => (self.transform_setne_rm8(i), false),
                Code::Shl_rm64_imm8 => (self.transform_shl_rm64_imm8(i), false),
                Code::Sub_rm32_r32 => (self.transform_sub_rm32_r32(i), false),
                Code::Sub_rm64_imm8 => (self.transform_sub_rm64_imm8(i), false),
                Code::Sub_rm64_imm32 => (self.transform_sub_rm64_imm32(i), false),
                Code::Sub_rm64_r64 => (self.transform_sub_rm64_r64(i), false),
                Code::Test_rm8_imm8 => (self.transform_test_rm8_imm8(i), false),
                Code::Test_rm8_r8 => (self.transform_test_rm8_r8(i), false),
                Code::Test_rm32_r32 => (self.transform_test_rm32_r32(i), false),
                Code::Test_rm64_r64 => (self.transform_test_rm64_r64(i), false),
                Code::Ud2 => (self.transform_ud2(i), true),
                Code::VEX_Vmovaps_ymmm256_ymm => (self.transform_vmovaps_ymmm256_ymm(i), false),
                Code::VEX_Vmovdqa_xmmm128_xmm => (self.transform_vmovdqa_xmmm128_xmm(i), false),
                Code::VEX_Vmovdqu_xmmm128_xmm => (self.transform_vmovdqu_xmmm128_xmm(i), false),
                Code::VEX_Vmovdqu_ymm_ymmm256 => (self.transform_vmovdqu_ymm_ymmm256(i), false),
                Code::VEX_Vmovdqu_ymmm256_ymm => (self.transform_vmovdqu_ymmm256_ymm(i), false),
                Code::VEX_Vmovq_xmm_rm64 => (self.transform_vmovq_xmm_rm64(i), false),
                Code::VEX_Vmovups_xmm_xmmm128 => (self.transform_vmovups_xmm_xmmm128(i), false),
                Code::VEX_Vmovups_xmmm128_xmm => (self.transform_vmovups_xmmm128_xmm(i), false),
                Code::VEX_Vmovups_ymm_ymmm256 => (self.transform_vmovups_ymm_ymmm256(i), false),
                Code::VEX_Vmovups_ymmm256_ymm => (self.transform_vmovups_ymmm256_ymm(i), false),
                Code::VEX_Vpshufd_xmm_xmmm128_imm8 => (self.transform_vpshufd_xmm_xmmm128_imm8(i), false),
                Code::VEX_Vpslldq_xmm_xmm_imm8 => (self.preserve(i), false),
                Code::VEX_Vpxor_xmm_xmm_xmmm128 => (self.transform_vpxor_xmm_xmm_xmmm128(i), false),
                Code::VEX_Vxorps_xmm_xmm_xmmm128 => (self.transform_vxorps_xmm_xmm_xmmm128(i), false),
                Code::VEX_Vxorps_ymm_ymm_ymmm256 => (self.transform_vxorps_ymm_ymm_ymmm256(i), false),
                Code::Wait => (self.preserve(i), false),
                Code::Xadd_rm32_r32 => (self.transform_xadd_rm32_r32(i), false),
                Code::Xchg_r32_EAX => (self.preserve(i), false),
                Code::Xchg_rm8_r8 => (self.transform_xchg_rm8_r8(i), false),
                Code::Xchg_rm32_r32 => (self.transform_xchg_rm32_r32(i), false),
                Code::Xor_AL_imm8 => (self.preserve(i), false),
                Code::Xor_EAX_imm32 => (self.preserve(i), false),
                Code::Xor_rm8_r8 => (self.transform_xor_rm8_r8(i), false),
                Code::Xor_rm32_r32 => (self.transform_xor_rm32_r32(i), false),
                _ => {
                    let opcode = &input[offset..(offset + i.len())];

                    return Err(RunError::UnknownInstruction(offset, opcode.into(), i));
                }
            };

            self.output_size += size;

            if end {
                break;
            }
        }

        Ok(base + start as u64)
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn recompile(&mut self, offset: usize, label: CodeLabel) -> Result<u64, RunError> {
        Err(RunError::UnsupportedArchError)
    }

    fn transform_add_r32_rm32(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("ADD r32, r/m32 with second operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_add_r64_rm64(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("ADD r/m64, r64 with second operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_add_rm8_imm8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("ADD r/m8, imm8 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_add_rm8_r8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("ADD r/m8, r8 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_add_rm32_r32(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("ADD r/m32, r32 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
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

    fn transform_add_rm64_imm32(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("ADD r/m64, imm32 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_add_rm64_r64(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("ADD r/m64, r64 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_and_rm8_imm8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("AND r/m8, imm8 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_and_rm32_imm8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("AND r/m32, imm8 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_and_rm32_imm32(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("AND r/m32, imm32 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_and_rm64_imm8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("AND r/m64, imm8 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_and_rm64_imm32(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("AND r/m64, imm32 with first operand as RIP-relative is not supported yet.");
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
            self.assembler.call(*label).unwrap();
        } else {
            let label = self.assembler.create_label();

            self.assembler.call(label).unwrap();
            self.jobs.push_back((dest, self.offset(dest), label));
        }

        15
    }

    fn transform_cmove_r64_rm64(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let dst = i.op0_register();
            let src = i.ip_rel_memory_address();
            let tmp = get_gpr64(Self::temp_register64(dst)).unwrap();

            if self.is_executable(src) {
                panic!("CMOVE r64, r/m64 with second operand from executable segment is not supported.");
            }

            // Transform to absolute address.
            self.assembler.push(tmp).unwrap();
            self.assembler.mov(tmp, src).unwrap();
            self.assembler
                .cmove(get_gpr64(dst).unwrap(), qword_ptr(tmp))
                .unwrap();
            self.assembler.pop(tmp).unwrap();

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_cmovne_r64_rm64(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let dst = i.op0_register();
            let src = i.ip_rel_memory_address();
            let tmp = get_gpr64(Self::temp_register64(dst)).unwrap();

            if self.is_executable(src) {
                panic!("CMOVNE r64, r/m64 with second operand from executable segment is not supported.");
            }

            // Transform to absolute address.
            self.assembler.push(tmp).unwrap();
            self.assembler.mov(tmp, src).unwrap();
            self.assembler
                .cmovne(get_gpr64(dst).unwrap(), qword_ptr(tmp))
                .unwrap();
            self.assembler.pop(tmp).unwrap();

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_cmp_r8_rm8(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let dst = i.op0_register();
            let src = i.ip_rel_memory_address();
            let tmp = get_gpr64(Self::temp_register64(dst)).unwrap();

            if self.is_executable(src) {
                panic!(
                    "CMP r8, r/m8 with second operand from executable segment is not supported."
                );
            }

            // Transform to absolute address.
            self.assembler.push(tmp).unwrap();
            self.assembler.mov(tmp, src).unwrap();
            self.assembler
                .cmp(get_gpr8(dst).unwrap(), byte_ptr(tmp))
                .unwrap();
            self.assembler.pop(tmp).unwrap();

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_cmp_rm8_imm8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let dst = i.ip_rel_memory_address();
            let src = i.immediate8();

            if self.is_executable(dst) {
                panic!(
                    "CMP r/m8, imm8 with first operand from executable segment is not supported."
                );
            }

            // Transform to absolute address.
            self.assembler.push(rax).unwrap();
            self.assembler.mov(rax, dst).unwrap();
            self.assembler.cmp(byte_ptr(rax), src as u32).unwrap();
            self.assembler.pop(rax).unwrap();

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_cmp_rm8_r8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("CMP r/m8, r8 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_cmp_rm32_imm8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let dst = i.ip_rel_memory_address();
            let src = i.immediate8();

            if self.is_executable(dst) {
                panic!(
                    "CMP r/m32, imm8 with first operand from executable segment is not supported."
                );
            }

            // Transform to absolute address.
            self.assembler.push(rax).unwrap();
            self.assembler.mov(rax, dst).unwrap();
            self.assembler.cmp(dword_ptr(rax), src as u32).unwrap();
            self.assembler.pop(rax).unwrap();

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_cmp_rm32_r32(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let src = i.op1_register();
            let dst = i.ip_rel_memory_address();
            let tmp = get_gpr64(Self::temp_register64(src)).unwrap();

            if self.is_executable(dst) {
                panic!(
                    "CMP r/m32, r32 with first operand from executable segment is not supported."
                );
            }

            // Transform to absolute address.
            self.assembler.push(tmp).unwrap();
            self.assembler.mov(tmp, dst).unwrap();
            self.assembler.cmp(get_gpr32(src).unwrap(), dword_ptr(tmp)).unwrap();
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

    fn transform_cmp_rm64_r64(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("CMP r/m64, r64 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_cmpsb_m8_m8(&mut self, i: Instruction) -> usize {
        if i.is_ip_rel_memory_operand() {
            panic!("CMPSB m8, m8 with RIP-relative addressing is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_dec_rm32(&mut self, i: Instruction) -> usize {
        // Check if operand use RIP-relative.
        if i.is_ip_rel_memory_operand() {
            panic!("DEC r/m32 with operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_dec_rm64(&mut self, i: Instruction) -> usize {
        // Check if operand use RIP-relative.
        if i.is_ip_rel_memory_operand() {
            panic!("DEC r/m64 with operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_imul_r32_rm32_imm8(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let dst = i.op0_register();
            let imm = i.immediate8();
            let src = i.ip_rel_memory_address();
            let tmp = get_gpr64(Self::temp_register64(dst)).unwrap();

            if self.is_executable(src) {
                panic!(
                    "IMUL r32, r/m32, imm8 with second operand from executable segment is not supported."
                );
            }

            // Transform to absolute address.
            self.assembler.push(tmp).unwrap();
            self.assembler.mov(tmp, src).unwrap();
            self.assembler.imul_3(get_gpr32(dst).unwrap(), dword_ptr(tmp), imm) .unwrap();
            self.assembler.pop(tmp).unwrap();

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_imul_r32_rm32_imm32(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let dst = i.op0_register();
            let imm = i.immediate32();
            let src = i.ip_rel_memory_address();
            let tmp = get_gpr64(Self::temp_register64(dst)).unwrap();

            if self.is_executable(src) {
                panic!(
                    "IMUL r32, r/m32, imm32 with second operand from executable segment is not supported."
                );
            }

            // Transform to absolute address.
            self.assembler.push(tmp).unwrap();
            self.assembler.mov(tmp, src).unwrap();
            self.assembler.imul_3(get_gpr32(dst).unwrap(), dword_ptr(tmp), imm).unwrap();
            self.assembler.pop(tmp).unwrap();

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_inc_rm32(&mut self, i: Instruction) -> usize {
        // Check if operand use RIP-relative.
        if i.is_ip_rel_memory_operand() {
            panic!("INC r/m32 with operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_inc_rm64(&mut self, i: Instruction) -> usize {
        // Check if operand use RIP-relative.
        if i.is_ip_rel_memory_operand() {
            panic!("INC r/m64 with operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_ja_rel(&mut self, i: Instruction) -> usize {
        let dst = i.near_branch64();

        if let Some(&label) = self.labels.get(&dst) {
            self.assembler.ja(label).unwrap();
        } else {
            let label = self.assembler.create_label();

            self.assembler.ja(label).unwrap();
            self.jobs.push_back((dst, self.offset(dst), label));
        }

        15
    }

    fn transform_jae_rel(&mut self, i: Instruction) -> usize {
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

    fn transform_jb_rel(&mut self, i: Instruction) -> usize {
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

    fn transform_jbe_rel(&mut self, i: Instruction) -> usize {
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

    fn transform_jg_rel(&mut self, i: Instruction) -> usize {
        let dst = i.near_branch64();

        if let Some(&label) = self.labels.get(&dst) {
            self.assembler.jg(label).unwrap();
        } else {
            let label = self.assembler.create_label();

            self.assembler.jg(label).unwrap();
            self.jobs.push_back((dst, self.offset(dst), label));
        }

        15
    }

    fn transform_jge_rel(&mut self, i: Instruction) -> usize {
        let dst = i.near_branch64();

        if let Some(&label) = self.labels.get(&dst) {
            self.assembler.jge(label).unwrap();
        } else {
            let label = self.assembler.create_label();

            self.assembler.jge(label).unwrap();
            self.jobs.push_back((dst, self.offset(dst), label));
        }

        15
    }

    fn transform_jl_rel(&mut self, i: Instruction) -> usize {
        let dst = i.near_branch64();

        if let Some(&label) = self.labels.get(&dst) {
            self.assembler.jl(label).unwrap();
        } else {
            let label = self.assembler.create_label();

            self.assembler.jl(label).unwrap();
            self.jobs.push_back((dst, self.offset(dst), label));
        }

        15
    }

    fn transform_jle_rel(&mut self, i: Instruction) -> usize {
        let dst = i.near_branch64();

        if let Some(&label) = self.labels.get(&dst) {
            self.assembler.jle(label).unwrap();
        } else {
            let label = self.assembler.create_label();

            self.assembler.jle(label).unwrap();
            self.jobs.push_back((dst, self.offset(dst), label));
        }

        15
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

    fn transform_jmp_rm64(&mut self, i: Instruction) -> usize {
        // Check if operand uses RIP-relative.
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

    fn transform_jns_rel(&mut self, i: Instruction) -> usize {
        let dst = i.near_branch64();

        if let Some(&label) = self.labels.get(&dst) {
            self.assembler.jns(label).unwrap();
        } else {
            let label = self.assembler.create_label();

            self.assembler.jns(label).unwrap();
            self.jobs.push_back((dst, self.offset(dst), label));
        }

        15
    }

    fn transform_jo_rel(&mut self, i: Instruction) -> usize {
        let dst = i.near_branch64();

        if let Some(&label) = self.labels.get(&dst) {
            self.assembler.jo(label).unwrap();
        } else {
            let label = self.assembler.create_label();

            self.assembler.jo(label).unwrap();
            self.jobs.push_back((dst, self.offset(dst), label));
        }
        2
    }

    fn transform_js_rel(&mut self, i: Instruction) -> usize {
        let dst = i.near_branch64();

        if let Some(&label) = self.labels.get(&dst) {
            self.assembler.js(label).unwrap();
        } else {
            let label = self.assembler.create_label();

            self.assembler.js(label).unwrap();
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

    fn transform_lea32(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.is_ip_rel_memory_operand() {
            // Check if second operand already recompiled.
            let dst = get_gpr64(i.op0_register()).unwrap();
            let src = i.ip_rel_memory_address();

            if let Some(&label) = self.labels.get(&src) {
                self.assembler.lea(dst, dword_ptr(label)).unwrap();
            } else {
                // Check which segment the second operand fall under.
                let segment = self.segment(src);

                if segment.flags.is_executable() {
                    let label = self.assembler.create_label();

                    self.assembler.lea(dst, dword_ptr(label)).unwrap();
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
                // Call function to execute code at source memory location
                self.assembler.call(src).unwrap();
                // Move the result to the destination register
                self.assembler.mov(get_gpr8(dst).unwrap(), byte_ptr(tmp)).unwrap();
            } else {
                // Transform to absolute address.
                self.assembler.push(tmp).unwrap();
                self.assembler.mov(tmp, src).unwrap();
                self.assembler
                    .mov(get_gpr8(dst).unwrap(), byte_ptr(tmp))
                    .unwrap();
                self.assembler.pop(tmp).unwrap();
            }

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_mov_r32_rm32(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let dst = i.op0_register();
            let src = i.ip_rel_memory_address();
            let dst64 = dst.full_register();
            let tmp = get_gpr64(Self::temp_register64(dst64)).unwrap();

            if self.is_executable(src) {
                // Call function to execute code at source memory location
                self.assembler.call(src).unwrap();
                // Move the result to the destination register
                self.assembler.mov(get_gpr32(dst).unwrap(), dword_ptr(tmp)).unwrap();
            } else {
                // Transform to absolute address.
                self.assembler.push(tmp).unwrap();
                self.assembler.mov(tmp, src).unwrap();
                self.assembler
                    .mov(get_gpr32(dst).unwrap(), dword_ptr(tmp))
                    .unwrap();
                self.assembler.pop(tmp).unwrap();
            }

            15 * 4
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
                // Call function to execute code at source memory location
                self.assembler.call(src).unwrap();
                // Move the result to the destination register
                self.assembler.mov(get_gpr64(dst).unwrap(), qword_ptr(tmp)).unwrap();
            } else {
                // Transform to absolute address.
                self.assembler.push(tmp).unwrap();
                self.assembler.mov(tmp, src).unwrap();
                self.assembler
                    .mov(get_gpr64(dst).unwrap(), qword_ptr(tmp))
                    .unwrap();
                self.assembler.pop(tmp).unwrap();
            }

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

    fn transform_mov_rm32_imm32(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let dst = i.ip_rel_memory_address();
            let src = i.immediate32();

            if self.is_executable(dst) {
                panic!(
                    "MOV r/m32, imm32 with first operand from executable segment is not supported."
                );
            }

            // Transform to absolute address.
            self.assembler.push(rax).unwrap();
            self.assembler.mov(rax, dst).unwrap();
            self.assembler.mov(dword_ptr(rax), src).unwrap();
            self.assembler.pop(rax).unwrap();

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_mov_rm64_imm32(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let dst = i.ip_rel_memory_address();
            let src = i.immediate32();

            if self.is_executable(dst) {
                panic!(
                    "MOV r/m64, imm32 with first operand from executable segment is not supported."
                );
            }

            // Transform to absolute address.
            self.assembler.push(rax).unwrap();
            self.assembler.mov(rax, dst).unwrap();
            self.assembler.mov(dword_ptr(rax), src).unwrap();
            self.assembler.pop(rax).unwrap();

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_mov_rm8_r8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let src = i.op1_register();
            let dst = i.ip_rel_memory_address();
            let tmp = get_gpr64(Self::temp_register64(src)).unwrap();

            if self.is_executable(dst) {
                panic!(
                    "MOV r/m8, r8 with first operand from executable segment is not supported."
                );
            }

            // Transform to absolute address.
            self.assembler.push(tmp).unwrap();
            self.assembler.mov(tmp, dst).unwrap();
            self.assembler
                .mov(byte_ptr(tmp), get_gpr8(src).unwrap())
                .unwrap();
            self.assembler.pop(tmp).unwrap();

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_mov_rm32_r32(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let src = i.op1_register();
            let dst = i.ip_rel_memory_address();
            let tmp = get_gpr64(Self::temp_register64(src)).unwrap();

            if self.is_executable(dst) {
                panic!(
                    "MOV r/m32, r32 with first operand from executable segment is not supported."
                );
            }

            // Transform to absolute address.
            self.assembler.push(tmp).unwrap();
            self.assembler.mov(tmp, dst).unwrap();
            self.assembler
                .mov(dword_ptr(tmp), get_gpr32(src).unwrap())
                .unwrap();
            self.assembler.pop(tmp).unwrap();

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_mov_rm64_r64(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let src = i.op1_register();
            let dst = i.ip_rel_memory_address();
            let tmp = get_gpr64(Self::temp_register64(src)).unwrap();

            if self.is_executable(dst) {
                panic!(
                    "MOV r/m64, r64 with first operand from executable segment is not supported."
                );
            }

            // Transform to absolute address.
            self.assembler.push(tmp).unwrap();
            self.assembler.mov(tmp, dst).unwrap();
            self.assembler
                .mov(qword_ptr(tmp), get_gpr64(src).unwrap())
                .unwrap();
            self.assembler.pop(tmp).unwrap();

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_movaps_xmm_xmmm128(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("MOVAPS xmm1, xmm2/m128 with second operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_movd_xmm_rm32(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("MOVD xmm1, r/m32 with second operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_movdqu_xmmm128_xmm(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("MOVDQU xmm2/m128, xmm1 with second operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_movsx_r32_rm8(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("MOVSX r32, r/m8 with second operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_movsxd_r32_rm32(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let dst = i.op0_register();
            let src = i.ip_rel_memory_address();
            let tmp = get_gpr64(Self::temp_register64(dst)).unwrap();

            if self.is_executable(src) {
                panic!("MOVSXD r64, r/m32 with second operand from executable segment is not supported.");
            }

            // Transform to absolute address.
            self.assembler.push(tmp).unwrap();
            self.assembler.mov(tmp, src).unwrap();
            self.assembler
                .movsxd(get_gpr32(dst).unwrap(), dword_ptr(tmp))
                .unwrap();
            self.assembler.pop(tmp).unwrap();

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_movsxd_r64_rm32(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            let dst = i.op0_register();
            let src = i.ip_rel_memory_address();
            let tmp = get_gpr64(Self::temp_register64(dst)).unwrap();

            if self.is_executable(src) {
                panic!("MOVSXD r64, r/m32 with second operand from executable segment is not supported.");
            }

            // Transform to absolute address.
            self.assembler.push(tmp).unwrap();
            self.assembler.mov(tmp, src).unwrap();
            self.assembler
                .movsxd(get_gpr64(dst).unwrap(), dword_ptr(tmp))
                .unwrap();
            self.assembler.pop(tmp).unwrap();

            15 * 4
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_movzx_r32_rm8(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("MOVZX r32, r/m8 with second operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_neg_rm32(&mut self, i: Instruction) -> usize {
        // Check if operand use RIP-relative.
        if i.is_ip_rel_memory_operand() {
            panic!("NEG r/m32 with operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_neg_rm64(&mut self, i: Instruction) -> usize {
        // Check if operand use RIP-relative.
        if i.is_ip_rel_memory_operand() {
            panic!("NEG r/m64 with operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_or_rm64_imm8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("OR r/m64, imm8 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_or_rm64_r64(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("OR r/m64, r64 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_outsb_dx_m8(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("OUTSB DX, m8 with second operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_pshufd_xmm_xmmm128_imm8(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!(
                "PSHUFD xmm1, xmm2/m128, imm8, with second operand as RIP-relative is not supported yet."
            );
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_push_rm32(&mut self, i: Instruction) -> usize {
        // Check if operand use RIP-relative.
        if i.is_ip_rel_memory_operand() {
            panic!("PUSH r/m32 with operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_push_rm64(&mut self, i: Instruction) -> usize {
        // Check if operand use RIP-relative.
        if i.is_ip_rel_memory_operand() {
            panic!("PUSH r/m64 with operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_sar_rm64_imm8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("SAR r/m64, imm8 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_sbb_rm32_imm8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("SBB r/m32, imm8 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_sete_rm8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("SETE r/m8 with operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_setne_rm8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("SETNE r/m8 with operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_shl_rm64_imm8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("SHL r/m64, imm8 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_sub_rm32_r32(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("SUB r/m32, r32 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_sub_rm64_imm8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("SUB r/m64, imm8 with first operand as RIP-relative is not supported yet.");
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

    fn transform_sub_rm64_r64(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("SUB r/m64, r64 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_test_rm8_imm8(&mut self, i: Instruction) -> usize {
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("TEST r/m8, imm8 with first operand as RIP-relative is not supported yet.");
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

    /// Will not compile on an non-x86_64 machine!
    #[cfg(target_arch = "x86_64")]
    fn transform_ud2(&mut self, i: Instruction) -> usize {
        let handler: extern "sysv64" fn(&mut Process, usize) -> ! = Process::handle_ud2;
        let handler: u64 = handler as u64;
        let proc: u64 = self.proc as u64;

        self.assembler.mov(rsi, self.offset(i.ip()) as u64).unwrap();
        self.assembler.mov(rdi, proc).unwrap();
        self.assembler.call(handler).unwrap();

        15 * 3
    }

    fn transform_vmovaps_ymmm256_ymm(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!(
                "VMOVAPS ymm2/m256, ymm1 with first operand as RIP-relative is not supported yet."
            );
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
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

    fn transform_vmovdqu_xmmm128_xmm(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!(
                "VMOVDQU xmm2/m128, xmm1 with first operand as RIP-relative is not supported yet."
            );
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_vmovdqu_ymm_ymmm256(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!(
                "VMOVDQU ymm1, ymm2/m256 with second operand as RIP-relative is not supported yet."
            );
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_vmovdqu_ymmm256_ymm(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!(
                "VMOVDQU ymm2/m256, ymm1 with first operand as RIP-relative is not supported yet."
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

    fn transform_vmovups_xmm_xmmm128(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!(
                "VMOVUPS xmm1, xmm2/m128 with second operand as RIP-relative is not supported yet."
            );
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_vmovups_xmmm128_xmm(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!(
                "VMOVUPS xmm2/m128, xmm1 with first operand as RIP-relative is not supported yet."
            );
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_vmovups_ymm_ymmm256(&mut self, i: Instruction) -> usize {
        // Check if second operand use RIP-relative.
        if i.op1_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!(
                "VMOVUPS ymm1, ymm2/m256 with second operand as RIP-relative is not supported yet."
            );
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_vmovups_ymmm256_ymm(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!(
                "VMOVUPS ymm2/m256, ymm1 with first operand as RIP-relative is not supported yet."
            );
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_vpshufd_xmm_xmmm128_imm8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!(
                "VPSLLDQ xmm1, xmm2/m128, imm8, with first operand as RIP-relative is not supported yet."
            );
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_vpxor_xmm_xmm_xmmm128(&mut self, i: Instruction) -> usize {
        // Check if third operand use RIP-relative.
        if i.op2_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!(
                "VPXOR xmm1, xmm1, xmm2/m128, with third operand as RIP-relative is not supported yet."
            );
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_vxorps_xmm_xmm_xmmm128(&mut self, i: Instruction) -> usize {
        // Check if third operand use RIP-relative.
        if i.op2_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!(
                "VXORPS xmm1, xmm1, xmm2/m128, with third operand as RIP-relative is not supported yet."
            );
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_vxorps_ymm_ymm_ymmm256(&mut self, i: Instruction) -> usize {
        // Check if third operand use RIP-relative.
        if i.op2_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!(
                "VXORPS ymm1, ymm1, ymm2/m256, with third operand as RIP-relative is not supported yet."
            );
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_xadd_rm32_r32(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("XADD r/m32, r32 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_xchg_rm8_r8(&mut self, i: Instruction) -> usize {
        // Either operand can be memory for this instruction so we need to check both of it.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("XCHG r/m8, r8 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_xchg_rm32_r32(&mut self, i: Instruction) -> usize {
        // Either operand can be memory for this instruction so we need to check both of it.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("XCHG r/m32, r32 with first operand as RIP-relative is not supported yet.");
        } else {
            self.assembler.add_instruction(i).unwrap();
            i.len()
        }
    }

    fn transform_xor_rm8_r8(&mut self, i: Instruction) -> usize {
        // Check if first operand use RIP-relative.
        if i.op0_kind() == OpKind::Memory && i.is_ip_rel_memory_operand() {
            panic!("XOR r/m8, r8 with first operand as RIP-relative is not supported yet.");
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
            // 64Bit | 32Bit | 16Bit | 8Bit starting Registers => 64Bit ending Registers (Non-64Bit ending registers return NONE error.)
            Register::R8 | Register::R8D | Register::R8W | Register::R8L => Register::R9,
            Register::R9 | Register::R9D | Register::R9W | Register::R9L => Register::R8,
            Register::R10 | Register::R10D | Register::R10W | Register::R10L => Register::R11,
            Register::R11 | Register::R11D | Register::R11W | Register::R11L => Register::R10,
            Register::R12 | Register::R12D | Register::R12W | Register::R12L => Register::R13,
            Register::R13 | Register::R13D | Register::R13W | Register::R13L => Register::R12,
            Register::R14 | Register::R14D | Register::R14W | Register::R14L => Register::R15,
            Register::R15 | Register::R15D | Register::R15W | Register::R15L => Register::R14,
            Register::RDI | Register::EDI | Register::DI | Register::DIL => Register::RSI,
            Register::RSI | Register::ESI | Register::SI | Register::SIL => Register::RDI,
            Register::RAX | Register::EAX | Register::AX | Register::AL => Register::RBX,
            Register::RBX | Register::EBX | Register::BX | Register::BL => Register::RAX,
            Register::RCX | Register::ECX | Register::CX | Register::CL => Register::RDX,
            Register::RDX | Register::EDX | Register::DX | Register::DL => Register::RCX,
            r => panic!("Register {:?} is not implemented yet.", r),
        }
    }

    fn offset(&self, addr: u64) -> usize {
        let base: u64 = self.input.as_ptr() as u64;

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
        self.ptr as usize
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
    UnsupportedArchError,
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
            Self::UnsupportedArchError => write!(f, "{} target is not supported", ARCH),
        }
    }
}
