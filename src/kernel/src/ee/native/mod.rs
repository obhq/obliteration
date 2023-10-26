use super::ExecutionEngine;
use crate::fs::{VPath, VPathBuf};
use crate::memory::Protections;
use crate::process::VThread;
use crate::rtld::{CodeWorkspaceError, Memory, Module, UnprotectSegmentError};
use crate::syscalls::{SysIn, SysOut, Syscalls};
use byteorder::{ByteOrder, LE};
use iced_x86::code_asm::{
    al, dword_ptr, ecx, get_gpr64, qword_ptr, r10, r11, r11d, r12, r13, r14, r15, r8, r9, rax, rbp,
    rbx, rcx, rdi, rdx, rsi, rsp, AsmRegister64, CodeAssembler,
};
use iced_x86::{Code, Decoder, DecoderOptions, Instruction, OpKind, Register};
use std::any::Any;
use std::mem::{size_of, transmute};
use std::sync::{Arc, OnceLock};
use thiserror::Error;

/// An implementation of [`ExecutionEngine`] for running the PS4 binary natively.
#[derive(Debug)]
pub struct NativeEngine {
    syscalls: OnceLock<Syscalls>,
    xsave_area: i32,
}

impl NativeEngine {
    pub fn new() -> Arc<Self> {
        let xsave_area = unsafe { std::arch::x86_64::__cpuid_count(0x0d, 0).ebx as i32 };

        if xsave_area == 0 {
            panic!("Your CPU does not support XSAVE instruction.");
        }

        assert!(xsave_area > 0);

        Arc::new(Self {
            syscalls: OnceLock::new(),
            xsave_area,
        })
    }

    fn patch_mod<E>(self: &Arc<Self>, module: &mut Module<E>) -> Result<usize, SetupModuleError>
    where
        E: ExecutionEngine,
    {
        let path = module.path();

        // Patch all executable sections.
        let mem = module.memory();
        let base = mem.addr();
        let mut count = 0;

        for (i, seg) in mem.segments().iter().enumerate() {
            if seg.program().is_none() || !seg.prot().contains(Protections::CPU_EXEC) {
                continue;
            }

            // Unprotect the segment.
            let mut seg = match unsafe { mem.unprotect_segment(i) } {
                Ok(v) => v,
                Err(e) => return Err(SetupModuleError::UnprotectSegmentFailed(i, e)),
            };

            // Patch segment.
            count += unsafe { self.patch_segment(path, base, mem, seg.as_mut()) }?;
        }

        Ok(count)
    }

    /// # Safety
    /// No other threads may execute the memory in the code workspace on `mem`.
    unsafe fn patch_segment(
        self: &Arc<Self>,
        module: &VPath,
        base: usize,
        mem: &Memory,
        mut seg: &mut [u8],
    ) -> Result<usize, SetupModuleError> {
        // Get start address of the code.
        if seg.starts_with(b"/libexec/ld-elf.so.1\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00") {
            seg = &mut seg[0x20..];
        }

        // Patch.
        let addr = seg.as_ptr() as usize;
        let offset = addr - base;
        let mut count = 0;
        let mut i = 0;

        while i < seg.len() {
            let target = &mut seg[i..];
            let offset = offset + i;
            let addr = addr + i;
            let res = if target.starts_with(&[0x48, 0xc7, 0xc0]) {
                // Possible of:
                // mov rax, imm32
                // mov r10, rcx
                // syscall
                self.patch_syscall(module, mem, target, offset, addr)?
            } else if target.starts_with(&[0xcd, 0x44, 0xba, 0xcc, 0xcc, 0xcc, 0xcc]) {
                // int 0x44
                // mov edx, 0xcccccccc
                self.patch_int44(module, mem, target, offset, addr)?
            } else if target[0] == 0xF0 // LOCK prefix
                || target[0] == 0xF2 // REPNE/REPNZ prefix
                || target[0] == 0xF3 // REP or REPE/REPZ prefix
                || target[0] == 0x64 // FS segment override
                || target[0] == 0x66 // Operand-size override prefix
                || target[0] == 0x67
            {
                self.patch(module, mem, target, offset, addr)?
            } else {
                None
            };

            match res {
                Some(v) => {
                    i += v;
                    count += 1;
                }
                None => i += 1,
            }
        }

        Ok(count)
    }

    unsafe fn patch(
        self: &Arc<Self>,
        module: &VPath,
        mem: &Memory,
        target: &mut [u8],
        offset: usize,
        addr: usize,
    ) -> Result<Option<usize>, SetupModuleError> {
        // Check if instruction is valid.
        let mut decoder = Decoder::with_ip(64, target, target.as_ptr() as _, DecoderOptions::AMD);
        let inst = decoder.decode();

        if inst.is_invalid() {
            return Ok(None);
        }

        // Check if it is the instruction we need to patch.
        if inst.segment_prefix() == Register::FS {
            self.patch_fs(module, mem, target, offset, addr, inst)
        } else {
            Ok(None)
        }
    }

    /// # Safety
    /// No other threads may execute the memory in the code workspace on `mem`.
    unsafe fn patch_syscall(
        self: &Arc<Self>,
        module: &VPath,
        mem: &Memory,
        target: &mut [u8],
        offset: usize,
        addr: usize,
    ) -> Result<Option<usize>, SetupModuleError> {
        // Check if "mov rax, imm32".
        if target.len() < 7 {
            return Ok(None);
        }

        // Check if next instructions are:
        // mov r10, rcx
        // syscall
        if !target[7..].starts_with(&[0x49, 0x89, 0xca, 0x0f, 0x05]) {
            return Ok(None);
        }

        // Build trampoline.
        let id = LE::read_u32(&target[3..]);
        let ret = addr + 7 + 5;
        let tp = match self.build_syscall_trampoline(mem, module, offset + 10, id, ret) {
            Ok(v) => v,
            Err(e) => return Err(SetupModuleError::BuildTrampolineFailed(offset, e)),
        };

        // Patch "mov rax, imm32" with "jmp rel32".
        let tp = match Self::get_relative_offset(addr + 5, tp) {
            Some(v) => v.to_ne_bytes(),
            None => return Err(SetupModuleError::WorkspaceTooFar),
        };

        target[0] = 0xe9;
        target[1] = tp[0];
        target[2] = tp[1];
        target[3] = tp[2];
        target[4] = tp[3];
        target[5] = 0xcc;
        target[6] = 0xcc;

        // Patch "mov r10, rcx" and "syscall" with "int3" to catch unknown jump.
        target[7] = 0xcc;
        target[8] = 0xcc;
        target[9] = 0xcc;
        target[10] = 0xcc;
        target[11] = 0xcc;

        Ok(Some(7 + 5))
    }

    /// # Safety
    /// No other threads may execute the memory in the code workspace on `mem`.
    unsafe fn patch_int44(
        self: &Arc<Self>,
        module: &VPath,
        mem: &Memory,
        target: &mut [u8],
        offset: usize,
        addr: usize,
    ) -> Result<Option<usize>, SetupModuleError> {
        // Build trampoline.
        let ret = addr + 2 + 5;
        let tp = match self.build_int44_trampoline(mem, module, offset, ret) {
            Ok(v) => v,
            Err(e) => return Err(SetupModuleError::BuildTrampolineFailed(offset, e)),
        };

        // Patch "int 0x44" with "nop".
        target[0] = 0x90;
        target[1] = 0x90;

        // Patch "mov edx, 0xcccccccc" with "jmp rel32".
        let tp = match Self::get_relative_offset(addr + 7, tp) {
            Some(v) => v.to_ne_bytes(),
            None => return Err(SetupModuleError::WorkspaceTooFar),
        };

        target[2] = 0xe9;
        target[3] = tp[0];
        target[4] = tp[1];
        target[5] = tp[2];
        target[6] = tp[3];

        Ok(Some(7))
    }

    unsafe fn patch_fs(
        self: &Arc<Self>,
        module: &VPath,
        mem: &Memory,
        target: &mut [u8],
        offset: usize,
        addr: usize,
        inst: Instruction,
    ) -> Result<Option<usize>, SetupModuleError> {
        // Check for memory operand.
        let mut found = false;

        for i in 0..inst.op_count() {
            if inst.op_kind(i) == OpKind::Memory {
                found = true;
                break;
            }
        }

        if !found {
            return Ok(None);
        }

        // Check if fixed displacement.
        if inst.memory_base() != Register::None || inst.memory_index() != Register::None {
            return Ok(None);
        }

        // AFAIK there are only QWORD PTR FS:[0x00] and QWORD PTR FS:[0x10].
        let disp = inst.memory_displacement64();

        if disp != 0x00 && disp != 0x10 {
            return Ok(None);
        }

        // Patch.
        let ret = addr + inst.len();

        match inst.code() {
            Code::Mov_r64_rm64 => {
                assert!(inst.len() >= 5);

                // Some samples of possible instructions:
                // mov rax,fs:[0] -> 64, 48, 8b, 04, 25, 00, 00, 00, 00
                let out = get_gpr64(inst.op0_register()).unwrap();
                let tp = match self.build_fs_trampoline(mem, disp, out, ret) {
                    Ok(v) => v,
                    Err(e) => return Err(SetupModuleError::BuildTrampolineFailed(offset, e)),
                };

                // Patch the target with "jmp rel32".
                let tp = match Self::get_relative_offset(addr + 5, tp) {
                    Some(v) => v.to_ne_bytes(),
                    None => return Err(SetupModuleError::WorkspaceTooFar),
                };

                target[0] = 0xe9;
                target[1] = tp[0];
                target[2] = tp[1];
                target[3] = tp[2];
                target[4] = tp[3];

                // Patch the remaining with "int3".
                for i in 5..inst.len() {
                    target[i] = 0xcc;
                }
            }
            _ => todo!(
                "'{}' ({:02x?}) at {:#x} on {}.",
                inst,
                &target[..inst.len()],
                offset,
                module
            ),
        }

        Ok(Some(inst.len()))
    }

    /// # Safety
    /// No other threads may execute the memory in the code workspace on `mem`.
    unsafe fn build_syscall_trampoline(
        self: &Arc<Self>,
        mem: &Memory,
        module: &VPath,
        offset: usize,
        id: u32,
        ret: usize,
    ) -> Result<usize, TrampolineError> {
        let mut asm = CodeAssembler::new(64).unwrap();

        assert_eq!(72, size_of::<SysIn>());
        assert_eq!(16, size_of::<SysOut>());

        // Create function prologue.
        asm.push(rbp).unwrap();
        asm.mov(rbp, rsp).unwrap();

        // Clear CF and save rFLAGS.
        asm.clc().unwrap();
        asm.pushfq().unwrap();
        asm.pop(r11).unwrap();
        asm.and(rsp, !15).unwrap(); // Make sure stack is aligned to 16 bytes boundary.

        // Create stack frame.
        asm.sub(rsp, 0x50 + 0x10).unwrap();
        asm.mov(rax, rsp).unwrap();

        // Save registers.
        asm.push(rdi).unwrap();
        asm.push(rsi).unwrap();
        asm.push(rbx).unwrap();
        asm.push(r12).unwrap();
        asm.push(rdx).unwrap();
        asm.push(r11).unwrap();

        // Setup input.
        let module = match mem.push_data(module.to_owned()) {
            Some(v) => v,
            None => return Err(TrampolineError::SpaceNotEnough),
        };

        asm.mov(rbx, rax).unwrap();
        asm.mov(dword_ptr(rbx), id).unwrap();
        asm.mov(rax, offset as u64).unwrap();
        asm.mov(qword_ptr(rbx + 0x08), rax).unwrap();
        asm.mov(rax, module as u64).unwrap();
        asm.mov(qword_ptr(rbx + 0x10), rax).unwrap();
        asm.mov(qword_ptr(rbx + 0x18), rdi).unwrap();
        asm.mov(qword_ptr(rbx + 0x20), rsi).unwrap();
        asm.mov(qword_ptr(rbx + 0x28), rdx).unwrap();
        asm.mov(qword_ptr(rbx + 0x30), rcx).unwrap();
        asm.mov(qword_ptr(rbx + 0x38), r8).unwrap();
        asm.mov(qword_ptr(rbx + 0x40), r9).unwrap();

        // Invoke our routine.
        let ee = match mem.push_data(self.clone()) {
            Some(v) => v,
            None => return Err(TrampolineError::SpaceNotEnough),
        };

        asm.mov(rax, rbx).unwrap();
        asm.add(rax, 0x50).unwrap();
        asm.mov(rdx, rax).unwrap();
        asm.mov(rsi, rbx).unwrap();
        asm.mov(rdi, Arc::as_ptr(&*ee) as u64).unwrap();
        asm.mov(rax, Self::syscall as u64).unwrap();
        asm.call(rax).unwrap();

        // Check error. This mimics the behavior of
        // https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/amd64/amd64/vm_machdep.c#L380.
        let mut err = asm.create_label();
        let mut restore = asm.create_label();

        asm.pop(r11).unwrap();
        asm.pop(rdx).unwrap();
        asm.test(rax, rax).unwrap();
        asm.jnz(err).unwrap();

        // Set output.
        asm.mov(rax, qword_ptr(rbx + 0x50)).unwrap();
        asm.mov(rdx, qword_ptr(rbx + 0x58)).unwrap();
        asm.jmp(restore).unwrap();

        // Set CF.
        asm.set_label(&mut err).unwrap();
        asm.or(r11, 1).unwrap();

        // Restore registers.
        asm.set_label(&mut restore).unwrap();
        asm.pop(r12).unwrap();
        asm.pop(rbx).unwrap();
        asm.pop(rsi).unwrap();
        asm.pop(rdi).unwrap();
        asm.mov(rcx, ret as u64).unwrap();
        asm.xor(r8, r8).unwrap();
        asm.xor(r9, r9).unwrap();
        asm.xor(r10, r10).unwrap();

        // Restore rFLAGS.
        asm.mov(r11d, r11d).unwrap(); // Clear the upper dword.
        asm.push(r11).unwrap();
        asm.popfq().unwrap();
        asm.leave().unwrap();

        self.write_trampoline(mem, ret, asm)
    }

    /// # Safety
    /// This method cannot be called from Rust.
    unsafe extern "sysv64" fn syscall(&self, i: &SysIn, o: &mut SysOut) -> i64 {
        self.syscalls.get().unwrap().exec(i, o)
    }

    /// # Safety
    /// No other threads may execute the memory in the code workspace on `mem`.
    unsafe fn build_int44_trampoline(
        self: &Arc<Self>,
        mem: &Memory,
        module: &VPath,
        offset: usize,
        ret: usize,
    ) -> Result<usize, TrampolineError> {
        let mut asm = CodeAssembler::new(64).unwrap();

        // Create stack frame.
        asm.push(rbp).unwrap();
        asm.mov(rbp, rsp).unwrap();
        asm.and(rsp, !15).unwrap(); // Make sure stack is aligned to 16 bytes boundary.
        asm.push(rdi).unwrap();
        asm.push(rax).unwrap();
        asm.push(rdx).unwrap();
        asm.push(rsi).unwrap();

        // Invoke our routine.
        let module = match mem.push_data(module.to_owned()) {
            Some(v) => v,
            None => return Err(TrampolineError::SpaceNotEnough),
        };

        let ee = match mem.push_data(self.clone()) {
            Some(v) => v,
            None => return Err(TrampolineError::SpaceNotEnough),
        };

        asm.mov(rdx, module as u64).unwrap();
        asm.mov(rsi, offset as u64).unwrap();
        asm.mov(rdi, Arc::as_ptr(&*ee) as u64).unwrap();
        asm.mov(rax, Self::int44 as u64).unwrap();
        asm.call(rax).unwrap();

        // Restore registers.
        asm.pop(rsi).unwrap();
        asm.pop(rdx).unwrap();
        asm.pop(rax).unwrap();
        asm.pop(rdi).unwrap();
        asm.leave().unwrap();

        self.write_trampoline(mem, ret, asm)
    }

    /// # Safety
    /// This method cannot be called from Rust.
    unsafe extern "sysv64" fn int44(&self, offset: usize, module: &VPathBuf) -> ! {
        panic!("Exiting with int 0x44 at {offset:#x} on {module}.");
    }

    unsafe fn build_fs_trampoline(
        self: &Arc<Self>,
        mem: &Memory,
        disp: u64,
        out: AsmRegister64,
        ret: usize,
    ) -> Result<usize, TrampolineError> {
        let mut asm = CodeAssembler::new(64).unwrap();

        // Create stack frame and save current state.
        asm.push(rbp).unwrap();
        asm.mov(rbp, rsp).unwrap();
        asm.pushfq().unwrap();
        asm.pop(out).unwrap();
        asm.and(rsp, !63).unwrap();
        asm.push(out).unwrap(); // rFLAGS.
        asm.push(out).unwrap(); // Output placeholder.
        asm.push(rax).unwrap();
        asm.push(rbx).unwrap();
        asm.push(rdi).unwrap();
        asm.push(rsi).unwrap();
        asm.push(rdx).unwrap();
        asm.push(rcx).unwrap();
        asm.push(r8).unwrap();
        asm.push(r9).unwrap();
        asm.push(r10).unwrap();
        asm.push(r11).unwrap();
        asm.push(r12).unwrap();
        asm.push(r13).unwrap();
        asm.push(r14).unwrap();
        asm.push(r15).unwrap();
        asm.mov(rbx, rsp).unwrap();
        asm.add(rbx, 14 * 8).unwrap(); // Output placeholder.

        // Save x87, SSE and AVX states.
        let xsave = (self.xsave_area + 15) & !15;

        asm.sub(rsp, xsave).unwrap();
        asm.mov(ecx, xsave).unwrap();
        asm.xor(al, al).unwrap();
        asm.mov(rdi, rsp).unwrap();
        asm.rep().stosb().unwrap();
        asm.mov(rdx, 0xFFFFFFFFFFFFFFFFu64).unwrap();
        asm.mov(rax, 0xFFFFFFFFFFFFFFFFu64).unwrap();
        asm.xsave64(iced_x86::code_asm::ptr(rsp)).unwrap();

        // Invoke our routine.
        let ee = match mem.push_data(self.clone()) {
            Some(v) => v,
            None => return Err(TrampolineError::SpaceNotEnough),
        };

        asm.mov(rsi, disp).unwrap();
        asm.mov(rdi, Arc::as_ptr(&*ee) as u64).unwrap();
        asm.mov(rax, Self::resolve_fs as u64).unwrap();
        asm.call(rax).unwrap();
        asm.mov(qword_ptr(rbx), rax).unwrap();

        // Restore x87, SSE and AVX states.
        asm.mov(rdx, 0xFFFFFFFFFFFFFFFFu64).unwrap();
        asm.mov(rax, 0xFFFFFFFFFFFFFFFFu64).unwrap();
        asm.xrstor64(iced_x86::code_asm::ptr(rsp)).unwrap();
        asm.add(rsp, xsave).unwrap();

        // Restore registers.
        asm.pop(r15).unwrap();
        asm.pop(r14).unwrap();
        asm.pop(r13).unwrap();
        asm.pop(r12).unwrap();
        asm.pop(r11).unwrap();
        asm.pop(r10).unwrap();
        asm.pop(r9).unwrap();
        asm.pop(r8).unwrap();
        asm.pop(rcx).unwrap();
        asm.pop(rdx).unwrap();
        asm.pop(rsi).unwrap();
        asm.pop(rdi).unwrap();
        asm.pop(rbx).unwrap();
        asm.pop(rax).unwrap();
        asm.pop(out).unwrap();
        asm.popfq().unwrap();
        asm.leave().unwrap();

        self.write_trampoline(mem, ret, asm)
    }

    /// # Safety
    /// This method cannot be called from Rust.
    unsafe extern "sysv64" fn resolve_fs(&self, disp: usize) -> usize {
        std::ptr::read_unaligned((VThread::current().pcb().fsbase() + disp) as _)
    }

    /// # Safety
    /// No other threads may execute the memory in the code workspace on `mem`.
    unsafe fn write_trampoline(
        &self,
        mem: &Memory,
        ret: usize,
        mut assembled: CodeAssembler,
    ) -> Result<usize, TrampolineError> {
        // Get workspace.
        let mut mem = match mem.code_workspace() {
            Ok(v) => v,
            Err(e) => return Err(TrampolineError::GetCodeWorkspaceFailed(e)),
        };

        // Align base address to 16 byte boundary for performance.
        let base = mem.addr();
        let offset = match base % 16 {
            0 => 0,
            v => 16 - v,
        };

        // Assemble.
        let addr = base + offset;
        let mut assembled = assembled.assemble(addr as u64).unwrap();

        // Manually JMP rel32 back to patched location.
        assembled.extend({
            // Calculate relative offset.
            let mut code = [0u8; 5];
            let v = match Self::get_relative_offset(addr + assembled.len() + code.len(), ret) {
                Some(v) => v.to_ne_bytes(),
                None => return Err(TrampolineError::ReturnTooFar),
            };

            // Write the instruction.
            code[0] = 0xe9;
            code[1] = v[0];
            code[2] = v[1];
            code[3] = v[2];
            code[4] = v[3];

            code
        });

        // Write the assembled code.
        let output = match mem.as_mut().get_mut(offset..) {
            Some(v) => v,
            None => return Err(TrampolineError::SpaceNotEnough),
        };

        if assembled.len() > output.len() {
            return Err(TrampolineError::SpaceNotEnough);
        }

        output[..assembled.len()].copy_from_slice(&assembled);

        // Seal the workspace.
        mem.seal(offset + assembled.len());

        Ok(addr)
    }

    fn get_relative_offset(from: usize, to: usize) -> Option<i32> {
        let offset = to.wrapping_sub(from) as isize;

        offset.try_into().ok()
    }
}

impl ExecutionEngine for NativeEngine {
    type RawFn = RawFn;
    type SetupModuleErr = SetupModuleError;
    type GetFunctionErr = GetFunctionError;

    fn set_syscalls(&self, v: Syscalls) {
        self.syscalls.set(v).unwrap();
    }

    fn setup_module(self: &Arc<Self>, md: &mut Module<Self>) -> Result<(), Self::SetupModuleErr> {
        self.patch_mod(md)?;
        Ok(())
    }

    unsafe fn get_function(
        self: &Arc<Self>,
        md: &Arc<Module<Self>>,
        addr: usize,
    ) -> Result<Arc<Self::RawFn>, Self::GetFunctionErr> {
        Ok(Arc::new(RawFn {
            md: md.clone(),
            addr,
        }))
    }
}

/// An implementation of [`super::RawFn`].
pub struct RawFn {
    #[allow(unused)]
    md: Arc<dyn Any + Send + Sync>, // Keep module alive.
    addr: usize,
}

impl super::RawFn for RawFn {
    fn addr(&self) -> usize {
        self.addr
    }

    unsafe fn exec1<R, A>(&self, a: A) -> R {
        let f: unsafe extern "sysv64" fn(A) -> R = transmute(self.addr);
        f(a)
    }
}

/// An implementation of [`ExecutionEngine::SetupModuleErr`].
#[derive(Debug, Error)]
pub enum SetupModuleError {
    #[error("cannot unprotect segment {0} -> {1}")]
    UnprotectSegmentFailed(usize, #[source] UnprotectSegmentError),

    #[error("cannot build a trampoline for {0:#x} -> {1}")]
    BuildTrampolineFailed(usize, #[source] TrampolineError),

    #[error("module workspace is too far")]
    WorkspaceTooFar,
}

/// Errors for trampoline building.
#[derive(Debug, Error)]
pub enum TrampolineError {
    #[error("cannot get code workspace -> {0}")]
    GetCodeWorkspaceFailed(#[source] CodeWorkspaceError),

    #[error("the remaining workspace is not enough")]
    SpaceNotEnough,

    #[error("the address to return is too far")]
    ReturnTooFar,
}

/// An implementation of [`ExecutionEngine::GetFunctionErr`].
#[derive(Debug, Error)]
pub enum GetFunctionError {}
