pub mod x64;
use std::error::Error;
use std::fmt::{Display, Formatter};

use iced_x86::code_asm::CodeLabel;
use iced_x86::Instruction;
use libc;

use super::Segment;

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
#[derive(Copy, Clone)]
pub enum LabelType {
    X64CodeLabel(CodeLabel),
}
#[derive(Debug)]
pub enum LabelTypeError {
    WrongLabelTypeError,
}
#[allow(unreachable_patterns)]
impl LabelType {
    fn as_x64_label(self) -> Result<CodeLabel, LabelTypeError> {
        match self {
            LabelType::X64CodeLabel(label) => Ok(label),
            _ => Err(LabelTypeError::WrongLabelTypeError),
        }
    }
}

pub trait Recompiler {
    fn run(self, starts: &[usize]) -> Result<(NativeCode, Vec<*const u8>), RunError>;

    fn recompile(&mut self, offset: usize, label_type: LabelType) -> Result<u64, RunError>;
    fn transform_add_r32_rm32(&mut self, i: Instruction) -> usize;
    fn transform_add_r64_rm64(&mut self, i: Instruction) -> usize;
    fn transform_add_rm8_imm8(&mut self, i: Instruction) -> usize;
    fn transform_add_rm8_r8(&mut self, i: Instruction) -> usize;

    fn transform_add_rm32_r32(&mut self, i: Instruction) -> usize;

    fn transform_add_rm64_imm8(&mut self, i: Instruction) -> usize;

    fn transform_add_rm64_imm32(&mut self, i: Instruction) -> usize;

    fn transform_add_rm64_r64(&mut self, i: Instruction) -> usize;

    fn transform_and_rm8_imm8(&mut self, i: Instruction) -> usize;

    fn transform_and_rm32_imm8(&mut self, i: Instruction) -> usize;

    fn transform_and_rm32_imm32(&mut self, i: Instruction) -> usize;

    fn transform_and_rm64_imm8(&mut self, i: Instruction) -> usize;

    fn transform_and_rm64_imm32(&mut self, i: Instruction) -> usize;

    fn transform_call_rm64(&mut self, i: Instruction) -> usize;

    fn transform_call_rel32(&mut self, i: Instruction) -> usize;

    fn transform_cmove_r64_rm64(&mut self, i: Instruction) -> usize;
    fn transform_cmovne_r64_rm64(&mut self, i: Instruction) -> usize;

    fn transform_cmp_r8_rm8(&mut self, i: Instruction) -> usize;

    fn transform_cmp_rm8_imm8(&mut self, i: Instruction) -> usize;

    fn transform_cmp_rm8_r8(&mut self, i: Instruction) -> usize;

    fn transform_cmp_rm32_imm8(&mut self, i: Instruction) -> usize;

    fn transform_cmp_rm32_r32(&mut self, i: Instruction) -> usize;

    fn transform_cmp_rm64_imm8(&mut self, i: Instruction) -> usize;

    fn transform_cmp_r64_rm64(&mut self, i: Instruction) -> usize;

    fn transform_cmp_rm64_r64(&mut self, i: Instruction) -> usize;

    fn transform_cmpsb_m8_m8(&mut self, i: Instruction) -> usize;

    fn transform_dec_rm32(&mut self, i: Instruction) -> usize;

    fn transform_dec_rm64(&mut self, i: Instruction) -> usize;
    fn transform_imul_r32_rm32_imm8(&mut self, i: Instruction) -> usize;

    fn transform_imul_r32_rm32_imm32(&mut self, i: Instruction) -> usize;

    fn transform_inc_rm32(&mut self, i: Instruction) -> usize;

    fn transform_inc_rm64(&mut self, i: Instruction) -> usize;

    fn transform_ja_rel(&mut self, i: Instruction) -> usize;

    fn transform_jae_rel(&mut self, i: Instruction) -> usize;
    fn transform_jb_rel(&mut self, i: Instruction) -> usize;
    fn transform_jbe_rel(&mut self, i: Instruction) -> usize;

    fn transform_je_rel(&mut self, i: Instruction) -> usize;

    fn transform_jg_rel(&mut self, i: Instruction) -> usize;

    fn transform_jge_rel(&mut self, i: Instruction) -> usize;

    fn transform_jl_rel(&mut self, i: Instruction) -> usize;

    fn transform_jle_rel(&mut self, i: Instruction) -> usize;

    fn transform_jmp_rel(&mut self, i: Instruction) -> usize;

    fn transform_jmp_rm64(&mut self, i: Instruction) -> usize;

    fn transform_jne_rel(&mut self, i: Instruction) -> usize;

    fn transform_jns_rel(&mut self, i: Instruction) -> usize;
    fn transform_jo_rel(&mut self, i: Instruction) -> usize;

    fn transform_js_rel(&mut self, i: Instruction) -> usize;

    fn transform_lea64(&mut self, i: Instruction) -> usize;

    fn transform_lea32(&mut self, i: Instruction) -> usize;

    fn transform_mov_r8_rm8(&mut self, i: Instruction) -> usize;

    fn transform_mov_r32_rm32(&mut self, i: Instruction) -> usize;

    fn transform_mov_r64_rm64(&mut self, i: Instruction) -> usize;

    fn transform_mov_rm8_imm8(&mut self, i: Instruction) -> usize;

    fn transform_mov_rm32_imm32(&mut self, i: Instruction) -> usize;

    fn transform_mov_rm64_imm32(&mut self, i: Instruction) -> usize;

    fn transform_mov_rm8_r8(&mut self, i: Instruction) -> usize;

    fn transform_mov_rm32_r32(&mut self, i: Instruction) -> usize;

    fn transform_mov_rm64_r64(&mut self, i: Instruction) -> usize;

    fn transform_movaps_xmm_xmmm128(&mut self, i: Instruction) -> usize;

    fn transform_movd_xmm_rm32(&mut self, i: Instruction) -> usize;
    fn transform_movdqu_xmmm128_xmm(&mut self, i: Instruction) -> usize;

    fn transform_movsx_r32_rm8(&mut self, i: Instruction) -> usize;

    fn transform_movsxd_r32_rm32(&mut self, i: Instruction) -> usize;

    fn transform_movsxd_r64_rm32(&mut self, i: Instruction) -> usize;
    fn transform_movzx_r32_rm8(&mut self, i: Instruction) -> usize;

    fn transform_neg_rm32(&mut self, i: Instruction) -> usize;

    fn transform_neg_rm64(&mut self, i: Instruction) -> usize;
    fn transform_or_rm64_imm8(&mut self, i: Instruction) -> usize;

    fn transform_or_rm64_r64(&mut self, i: Instruction) -> usize;
    fn transform_outsb_dx_m8(&mut self, i: Instruction) -> usize;

    fn transform_pshufd_xmm_xmmm128_imm8(&mut self, i: Instruction) -> usize;

    fn transform_push_rm32(&mut self, i: Instruction) -> usize;
    fn transform_push_rm64(&mut self, i: Instruction) -> usize;

    fn transform_sar_rm64_imm8(&mut self, i: Instruction) -> usize;

    fn transform_sbb_rm32_imm8(&mut self, i: Instruction) -> usize;

    fn transform_sete_rm8(&mut self, i: Instruction) -> usize;

    fn transform_setne_rm8(&mut self, i: Instruction) -> usize;

    fn transform_shl_rm64_imm8(&mut self, i: Instruction) -> usize;

    fn transform_sub_rm32_r32(&mut self, i: Instruction) -> usize;
    fn transform_sub_rm64_imm8(&mut self, i: Instruction) -> usize;

    fn transform_sub_rm64_imm32(&mut self, i: Instruction) -> usize;
    fn transform_sub_rm64_r64(&mut self, i: Instruction) -> usize;

    fn transform_test_rm8_imm8(&mut self, i: Instruction) -> usize;

    fn transform_test_rm8_r8(&mut self, i: Instruction) -> usize;
    fn transform_test_rm32_r32(&mut self, i: Instruction) -> usize;

    fn transform_test_rm64_r64(&mut self, i: Instruction) -> usize;
    fn transform_ud2(&mut self, i: Instruction) -> usize;

    fn transform_vmovaps_ymmm256_ymm(&mut self, i: Instruction) -> usize;
    fn transform_vmovdqa_xmmm128_xmm(&mut self, i: Instruction) -> usize;

    fn transform_vmovdqu_xmmm128_xmm(&mut self, i: Instruction) -> usize;

    fn transform_vmovdqu_ymm_ymmm256(&mut self, i: Instruction) -> usize;
    fn transform_vmovdqu_ymmm256_ymm(&mut self, i: Instruction) -> usize;

    fn transform_vmovq_xmm_rm64(&mut self, i: Instruction) -> usize;

    fn transform_vmovups_xmm_xmmm128(&mut self, i: Instruction) -> usize;
    fn transform_vmovups_xmmm128_xmm(&mut self, i: Instruction) -> usize;
    fn transform_vmovups_ymm_ymmm256(&mut self, i: Instruction) -> usize;

    fn transform_vmovups_ymmm256_ymm(&mut self, i: Instruction) -> usize;

    fn transform_vpshufd_xmm_xmmm128_imm8(&mut self, i: Instruction) -> usize;

    fn transform_vpxor_xmm_xmm_xmmm128(&mut self, i: Instruction) -> usize;

    fn transform_vxorps_xmm_xmm_xmmm128(&mut self, i: Instruction) -> usize;

    fn transform_vxorps_ymm_ymm_ymmm256(&mut self, i: Instruction) -> usize;

    fn transform_xadd_rm32_r32(&mut self, i: Instruction) -> usize;

    fn transform_xchg_rm8_r8(&mut self, i: Instruction) -> usize;

    fn transform_xchg_rm32_r32(&mut self, i: Instruction) -> usize;

    fn transform_xor_rm8_r8(&mut self, i: Instruction) -> usize;

    fn transform_xor_rm32_r32(&mut self, i: Instruction) -> usize;

    #[cfg(target_arch = "x86_64")]
    fn preserve(&mut self, i: Instruction) -> usize;

    fn offset(&self, addr: u64) -> usize;
    fn is_executable(&self, addr: u64) -> bool;
    fn segment(&self, addr: u64) -> &Segment;
    fn aligned_output_size(&self) -> usize;

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
