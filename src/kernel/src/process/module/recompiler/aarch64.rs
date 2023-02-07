use super::{LabelType, NativeCode, Recompiler, RunError};
use crate::process::module::Segment;
use crate::process::Process;
use iced_x86::Instruction;

pub struct Aarch64Emitter<'input> {
    proc: *mut Process,
    input: &'input [u8],
    segments: Vec<Segment>,
    output_size: usize,
}

impl<'input> Aarch64Emitter<'input> {
    pub fn new(proc: *mut Process, input: &'input [u8], segments: Vec<Segment>) -> Self {
        Self {
            proc,
            input,
            segments,
            output_size: 0,
        }
    }
}

impl Recompiler for Aarch64Emitter<'_> {
    fn run(mut self, starts: &[usize]) -> Result<(NativeCode, Vec<*const u8>), RunError> {
        unimplemented!();
    }

    fn recompile(&mut self, offset: usize, label_type: LabelType) -> Result<u64, RunError> {
        unimplemented!();
    }

    fn transform_add_r32_rm32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_add_r64_rm64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_add_rm8_imm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_add_rm8_r8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_add_rm32_r32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_add_rm64_imm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_add_rm64_imm32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_add_rm64_r64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_and_rm8_imm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_and_rm32_imm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_and_rm32_imm32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_and_rm64_imm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_and_rm64_imm32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_call_rm64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_call_rel32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_cmove_r32_rm32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_cmove_r64_rm64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_cmovne_r64_rm64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_cmp_r8_rm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_cmp_rm8_imm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_cmp_rm8_r8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_cmp_rm32_imm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_cmp_rm32_r32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_cmp_rm64_imm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_cmp_r64_rm64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_cmp_rm64_r64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_cmpsb_m8_m8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_dec_rm32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_dec_rm64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_imul_r32_rm32_imm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_imul_r32_rm32_imm32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_inc_rm32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_inc_rm64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_ja_rel(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_jae_rel(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_jb_rel(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_jbe_rel(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_je_rel(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_jg_rel(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_jge_rel(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_jl_rel(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_jle_rel(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_jmp_rel(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_jmp_rm64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_jne_rel(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_jns_rel(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_jo_rel(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_js_rel(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_lea64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_lea32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_mov_r8_rm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_mov_r32_rm32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_mov_r64_rm64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_mov_rm8_imm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_mov_rm32_imm32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_mov_rm64_imm32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_mov_rm8_r8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_mov_rm32_r32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_mov_rm64_r64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_movaps_xmm_xmmm128(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_movd_xmm_rm32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_movdqu_xmmm128_xmm(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_movsx_r32_rm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_movsxd_r32_rm32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_movsxd_r64_rm32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_movzx_r32_rm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_neg_rm32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_neg_rm64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_or_rm64_imm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_or_rm64_r64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_outsb_dx_m8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_pshufd_xmm_xmmm128_imm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_push_rm32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_push_rm64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_sar_rm64_imm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_sbb_rm32_imm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_sete_rm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_setne_rm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_shl_rm64_imm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_sub_rm32_r32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_sub_rm64_imm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_sub_rm64_imm32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_sub_rm64_r64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_test_rm8_imm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_test_rm8_r8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_test_rm32_r32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_test_rm64_r64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_ud2(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_vmovaps_ymmm256_ymm(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_vmovdqa_xmmm128_xmm(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_vmovdqu_xmmm128_xmm(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_vmovdqu_ymm_ymmm256(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_vmovdqu_ymmm256_ymm(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_vmovq_xmm_rm64(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_vmovss_m32_xmm(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_vmovss_xmm_m32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_vmovups_xmm_xmmm128(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_vmovups_xmmm128_xmm(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_vmovups_ymm_ymmm256(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_vmovups_ymmm256_ymm(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_vpshufd_xmm_xmmm128_imm8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_vpxor_xmm_xmm_xmmm128(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_vxorps_xmm_xmm_xmmm128(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_vxorps_ymm_ymm_ymmm256(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_xadd_rm32_r32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_xchg_rm8_r8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_xchg_rm32_r32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_xor_rm8_r8(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn transform_xor_rm32_r32(&mut self, i: Instruction) -> usize {
        unimplemented!();
    }

    fn offset(&self, addr: u64) -> usize {
        unimplemented!();
    }

    fn is_executable(&self, addr: u64) -> bool {
        unimplemented!();
    }

    fn segment(&self, addr: u64) -> &Segment {
        unimplemented!();
    }

    fn aligned_output_size(&self) -> usize {
        unimplemented!();
    }
}
