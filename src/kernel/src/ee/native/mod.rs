use super::ExecutionEngine;
use crate::fs::path::{VPath, VPathBuf};
use crate::memory::{MprotectError, Protections};
use crate::rtld::{Memory, Module, RuntimeLinker};
use crate::syscalls::{Input, Output, Syscalls};
use byteorder::{ByteOrder, LE};
use iced_x86::code_asm::{
    dword_ptr, qword_ptr, r10, r11, r11d, r12, r8, r9, rax, rbp, rbx, rcx, rdi, rdx, rsi, rsp,
    CodeAssembler,
};
use std::error::Error;
use std::mem::{size_of, transmute};
use std::ptr::null_mut;
use std::sync::RwLock;
use thiserror::Error;

/// An implementation of [`ExecutionEngine`] for running the PS4 binary natively.
pub struct NativeEngine<'a, 'b: 'a> {
    rtld: &'a RwLock<RuntimeLinker<'b>>,
    syscalls: &'a Syscalls<'a, 'b>,
}

impl<'a, 'b: 'a> NativeEngine<'a, 'b> {
    pub fn new(rtld: &'a RwLock<RuntimeLinker<'b>>, syscalls: &'a Syscalls<'a, 'b>) -> Self {
        Self { rtld, syscalls }
    }

    /// # SAFETY
    /// No other threads may read or write any module memory.
    pub unsafe fn patch_mods(&mut self) -> Result<Vec<(VPathBuf, usize)>, PatchModsError> {
        let ld = self.rtld.read().unwrap();
        let mut counts: Vec<(VPathBuf, usize)> = Vec::new();

        for module in ld.list() {
            let count = self.patch_mod(module)?;
            let path: VPathBuf = module.image().name().try_into().unwrap();

            counts.push((path, count));
        }

        Ok(counts)
    }

    fn syscalls(&self) -> *const Syscalls<'a, 'b> {
        self.syscalls
    }

    /// # Safety
    /// No other threads may access the memory of `module`.
    unsafe fn patch_mod(&self, module: &Module) -> Result<usize, PatchModsError> {
        let path: VPathBuf = module.image().name().try_into().unwrap();

        // Patch all executable sections.
        let mem = module.memory();
        let base = mem.addr();
        let mut count = 0;

        for (i, seg) in mem.segments().iter().enumerate() {
            if seg.program().is_none() || !seg.prot().contains(Protections::CPU_EXEC) {
                continue;
            }

            // Unprotect the segment.
            let mut seg = match mem.unprotect_segment(i) {
                Ok(v) => v,
                Err(e) => return Err(PatchModsError::UnprotectMemoryFailed(path, e)),
            };

            // Patch segment.
            count += self.patch_segment(&path, base, mem, seg.as_mut())?;
        }

        Ok(count)
    }

    /// # Safety
    /// No other threads may execute the memory in the code workspace on `mem`.
    unsafe fn patch_segment(
        &self,
        module: &VPath,
        base: usize,
        mem: &Memory,
        seg: &mut [u8],
    ) -> Result<usize, PatchModsError> {
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

    /// # Safety
    /// No other threads may execute the memory in the code workspace on `mem`.
    unsafe fn patch_syscall(
        &self,
        module: &VPath,
        mem: &Memory,
        target: &mut [u8],
        offset: usize,
        addr: usize,
    ) -> Result<Option<usize>, PatchModsError> {
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
            Err(e) => {
                return Err(PatchModsError::BuildTrampolineFailed(
                    module.to_owned(),
                    offset,
                    e,
                ))
            }
        };

        // Patch "mov rax, imm32" with "jmp rel32".
        let tp = match Self::get_relative_offset(addr + 5, tp) {
            Some(v) => v.to_ne_bytes(),
            None => return Err(PatchModsError::WorkspaceTooFar(module.to_owned())),
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
        &self,
        module: &VPath,
        mem: &Memory,
        target: &mut [u8],
        offset: usize,
        addr: usize,
    ) -> Result<Option<usize>, PatchModsError> {
        // Build trampoline.
        let ret = addr + 2 + 5;
        let tp = match self.build_int44_trampoline(mem, module, offset, ret) {
            Ok(v) => v,
            Err(e) => {
                return Err(PatchModsError::BuildTrampolineFailed(
                    module.to_owned(),
                    offset,
                    e,
                ))
            }
        };

        // Patch "int 0x44" with "nop".
        target[0] = 0x90;
        target[1] = 0x90;

        // Patch "mov edx, 0xcccccccc" with "jmp rel32".
        let tp = match Self::get_relative_offset(addr + 7, tp) {
            Some(v) => v.to_ne_bytes(),
            None => return Err(PatchModsError::WorkspaceTooFar(module.to_owned())),
        };

        target[2] = 0xe9;
        target[3] = tp[0];
        target[4] = tp[1];
        target[5] = tp[2];
        target[6] = tp[3];

        Ok(Some(7))
    }

    /// # Safety
    /// No other threads may execute the memory in the code workspace on `mem`.
    unsafe fn build_syscall_trampoline(
        &self,
        mem: &Memory,
        module: &VPath,
        offset: usize,
        id: u32,
        ret: usize,
    ) -> Result<usize, TrampolineError> {
        let mut asm = CodeAssembler::new(64).unwrap();

        assert_eq!(72, size_of::<Input>());
        assert_eq!(16, size_of::<Output>());

        // Create function prologue.
        asm.push(rbp).unwrap();
        asm.mov(rbp, rsp).unwrap();

        // Clear CF and save rFLAGS.
        asm.clc().unwrap();
        asm.pushfq().unwrap();
        asm.pop(r11).unwrap();
        asm.and(rsp, !15).unwrap(); // Make sure stack is align to 16 bytes boundary.

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
        asm.mov(rax, rbx).unwrap();
        asm.add(rax, 0x50).unwrap();
        asm.mov(rdx, rax).unwrap();
        asm.mov(rsi, rbx).unwrap();
        asm.mov(rdi, self.syscalls() as u64).unwrap();
        asm.mov(rax, Syscalls::invoke as u64).unwrap();
        asm.call(rax).unwrap();

        // Check error. This mimic the behavior of
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
    /// No other threads may execute the memory in the code workspace on `mem`.
    unsafe fn build_int44_trampoline(
        &self,
        mem: &Memory,
        module: &VPath,
        offset: usize,
        ret: usize,
    ) -> Result<usize, TrampolineError> {
        let mut asm = CodeAssembler::new(64).unwrap();

        // Create stack frame.
        asm.push(rbp).unwrap();
        asm.mov(rbp, rsp).unwrap();
        asm.and(rsp, !15).unwrap(); // Make sure stack is align to 16 bytes boundary.
        asm.push(rdi).unwrap();
        asm.push(rax).unwrap();
        asm.push(rdx).unwrap();
        asm.push(rsi).unwrap();

        // Invoke our routine.
        let module = match mem.push_data(module.to_owned()) {
            Some(v) => v,
            None => return Err(TrampolineError::SpaceNotEnough),
        };

        asm.mov(rdx, module as u64).unwrap();
        asm.mov(rsi, offset as u64).unwrap();
        asm.mov(rdi, self.syscalls() as u64).unwrap();
        asm.mov(rax, Syscalls::int44 as u64).unwrap();
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
            Err(e) => return Err(TrampolineError::UnprotectWorkspaceFailed(e)),
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

    extern "sysv64" fn exit() {
        todo!()
    }
}

impl<'a, 'b> ExecutionEngine for NativeEngine<'a, 'b> {
    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        // Get eboot.bin.
        let ld = self.rtld.read().unwrap();
        let eboot = ld.app();

        if eboot.image().dynamic().is_none() {
            todo!("A statically linked eboot.bin is not supported yet.");
        }

        // Get boot module.
        let boot = ld.kernel().unwrap();

        // Get entry point.
        let mem = boot.memory().as_ref();
        let entry: EntryPoint = unsafe { transmute(mem[boot.entry().unwrap()..].as_ptr()) };

        drop(ld);

        // TODO: Check how the actual binary read its argument.
        // Setup arguments.
        let mut argv: Vec<*mut u8> = Vec::new();
        let mut arg1 = b"prog\0".to_vec();

        argv.push(arg1.as_mut_ptr());
        argv.push(null_mut());

        // Invoke entry point.
        let mut arg = Arg {
            argc: (argv.len() as i32) - 1,
            argv: argv.as_mut_ptr(),
        };

        entry(&mut arg, Self::exit);

        Ok(())
    }
}

type EntryPoint = extern "sysv64" fn(*mut Arg, extern "sysv64" fn());

#[repr(C)]
struct Arg {
    pub argc: i32,
    pub argv: *mut *mut u8,
}

/// Errors for [`NativeEngine::patch_mods()`].
#[derive(Debug, Error)]
pub enum PatchModsError {
    #[error("cannot unprotect memory {0}")]
    UnprotectMemoryFailed(VPathBuf, #[source] MprotectError),

    #[error("cannot build a trampoline for {1:#018x} on {0}")]
    BuildTrampolineFailed(VPathBuf, usize, #[source] TrampolineError),

    #[error("workspace of {0} is too far")]
    WorkspaceTooFar(VPathBuf),
}

/// Errors for trampoline building.
#[derive(Debug, Error)]
pub enum TrampolineError {
    #[error("cannot unprotect code workspace")]
    UnprotectWorkspaceFailed(#[source] MprotectError),

    #[error("the remaining workspace is not enough")]
    SpaceNotEnough,

    #[error("the address to return is too far")]
    ReturnTooFar,
}
