use super::ExecutionEngine;
use crate::fs::path::{VPath, VPathBuf};
use crate::memory::{MprotectError, Protections};
use crate::module::{Module, ModuleManager, ModuleWorkspace};
use crate::syscalls::Syscalls;
use byteorder::{ByteOrder, LE};
use iced_x86::code_asm::{rax, rbp, rcx, rdi, rdx, rsi, rsp, CodeAssembler};
use std::error::Error;
use std::mem::transmute;
use std::ptr::null_mut;
use thiserror::Error;

/// An implementation of [`ExecutionEngine`] for running the PS4 binary natively.
pub struct NativeEngine<'a, 'b: 'a> {
    modules: &'a ModuleManager<'b>,
    syscalls: &'a Syscalls,
}

impl<'a, 'b: 'a> NativeEngine<'a, 'b> {
    pub fn new(modules: &'a ModuleManager<'b>, syscalls: &'a Syscalls) -> Self {
        Self { modules, syscalls }
    }

    /// # SAFETY
    /// No other threads may read or write any module memory.
    pub unsafe fn patch_mods(&mut self) -> Result<Vec<(VPathBuf, usize)>, PatchModsError> {
        let mut counts: Vec<(VPathBuf, usize)> = Vec::new();

        self.modules.for_each(|module| {
            let count = self.patch_mod(module)?;
            let path: VPathBuf = module.image().name().try_into().unwrap();

            counts.push((path, count));
            Ok(())
        })?;

        Ok(counts)
    }

    fn syscalls(&self) -> *const Syscalls {
        self.syscalls
    }

    fn patch_mod(&self, module: &Module) -> Result<usize, PatchModsError> {
        let path: VPathBuf = module.image().name().try_into().unwrap();

        // Get the module memory.
        let mut mem = match unsafe { module.memory().unprotect() } {
            Ok(v) => v,
            Err(e) => return Err(PatchModsError::UnprotectMemoryFailed(path, e)),
        };

        // Patch all executable sections.
        let base = mem.addr();
        let mut count = 0;

        for seg in module.memory().segments() {
            if !seg.prot().contains(Protections::CPU_EXEC) {
                continue;
            }

            // Patch segment.
            let start = seg.start();
            let mem = &mut mem[start..(start + seg.len())];

            count += self.patch_segment(&path, base, module.memory().workspace(), mem)?;
        }

        Ok(count)
    }

    fn patch_segment(
        &self,
        module: &VPath,
        base: usize,
        wp: &ModuleWorkspace,
        seg: &mut [u8],
    ) -> Result<usize, PatchModsError> {
        let addr = seg.as_ptr() as usize;
        let offset = addr - base;
        let mut count = 0;
        let mut i = 0;

        while i < seg.len() {
            let mem = &mut seg[i..];
            let offset = offset + i;
            let addr = addr + i;
            let res = if mem.starts_with(&[0x48, 0xc7, 0xc0]) {
                // Possible of:
                // mov rax, imm32
                // mov r10, rcx
                // syscall
                self.patch_syscall(module, wp, mem, offset, addr)?
            } else if mem.starts_with(&[0xcd, 0x44, 0xba, 0xcc, 0xcc, 0xcc, 0xcc]) {
                // int 0x44
                // mov edx, 0xcccccccc
                self.patch_int44(module, wp, mem, offset, addr)?
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

    fn patch_syscall(
        &self,
        module: &VPath,
        wp: &ModuleWorkspace,
        mem: &mut [u8],
        offset: usize,
        addr: usize,
    ) -> Result<Option<usize>, PatchModsError> {
        // Check if "mov rax, imm32".
        if mem.len() < 7 {
            return Ok(None);
        }

        // Check if next instructions are:
        // mov r10, rcx
        // syscall
        if !mem[7..].starts_with(&[0x49, 0x89, 0xca, 0x0f, 0x05]) {
            return Ok(None);
        }

        // Build trampoline.
        let id = LE::read_u32(&mem[3..]);
        let ret = addr + 7 + 5;
        let tp = match self.build_syscall_trampoline(wp, module, offset + 10, id, ret) {
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

        mem[0] = 0xe9;
        mem[1] = tp[0];
        mem[2] = tp[1];
        mem[3] = tp[2];
        mem[4] = tp[3];
        mem[5] = 0xcc;
        mem[6] = 0xcc;

        // Patch "mov r10, rcx" and "syscall" with "int3" to catch unknown jump.
        mem[7] = 0xcc;
        mem[8] = 0xcc;
        mem[9] = 0xcc;
        mem[10] = 0xcc;
        mem[11] = 0xcc;

        Ok(Some(7 + 5))
    }

    fn patch_int44(
        &self,
        module: &VPath,
        wp: &ModuleWorkspace,
        mem: &mut [u8],
        offset: usize,
        addr: usize,
    ) -> Result<Option<usize>, PatchModsError> {
        // Build trampoline.
        let ret = addr + 2 + 5;
        let tp = match self.build_int44_trampoline(wp, module, offset, ret) {
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
        mem[0] = 0x90;
        mem[1] = 0x90;

        // Patch "mov edx, 0xcccccccc" with "jmp rel32".
        let tp = match Self::get_relative_offset(addr + 7, tp) {
            Some(v) => v.to_ne_bytes(),
            None => return Err(PatchModsError::WorkspaceTooFar(module.to_owned())),
        };

        mem[2] = 0xe9;
        mem[3] = tp[0];
        mem[4] = tp[1];
        mem[5] = tp[2];
        mem[6] = tp[3];

        Ok(Some(7))
    }

    fn build_syscall_trampoline(
        &self,
        wp: &ModuleWorkspace,
        module: &VPath,
        offset: usize,
        id: u32,
        ret: usize,
    ) -> Result<usize, TrampolineError> {
        let mut asm = CodeAssembler::new(64).unwrap();

        // Create stack frame.
        asm.push(rbp).unwrap();
        asm.mov(rbp, rsp).unwrap();
        asm.and(rsp, !15).unwrap(); // Make sure stack is align to 16 bytes boundary.

        // Invoke our routine.
        let module = match wp.push(module.to_owned()) {
            Some(v) => v,
            None => return Err(TrampolineError::SpaceNotEnough),
        };

        asm.mov(rcx, module as u64).unwrap();
        asm.mov(rdx, offset as u64).unwrap();
        asm.mov(rsi, id as u64).unwrap();
        asm.mov(rdi, self.syscalls() as u64).unwrap();
        asm.mov(rax, Syscalls::unimplemented as u64).unwrap();
        asm.call(rax).unwrap();

        // Restore registers.
        asm.leave().unwrap();

        self.write_trampoline(wp, ret, asm)
    }

    fn build_int44_trampoline(
        &self,
        wp: &ModuleWorkspace,
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
        let module = match wp.push(module.to_owned()) {
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

        self.write_trampoline(wp, ret, asm)
    }

    fn write_trampoline(
        &self,
        wp: &ModuleWorkspace,
        ret: usize,
        mut assembled: CodeAssembler,
    ) -> Result<usize, TrampolineError> {
        let mut mem = wp.memory();

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
        let output = match mem.as_mut_slice().get_mut(offset..) {
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
        // Get boot module.
        let path: &VPath = "/system/common/lib/libkernel.sprx".try_into().unwrap();
        let boot = match self.modules.get_mod(path) {
            Some(v) => v,
            None => self.modules.get_eboot(),
        };

        // Get entry point.
        let mem = boot.memory().as_ref();
        let entry: EntryPoint =
            unsafe { transmute(mem[boot.image().entry_addr().unwrap()..].as_ptr()) };

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
    #[error("the remaining workspace is not enough")]
    SpaceNotEnough,

    #[error("the address to return is too far")]
    ReturnTooFar,
}
