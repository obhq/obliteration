use super::ExecutionEngine;
use crate::fs::path::{VPath, VPathBuf};
use crate::memory::{MprotectError, Protections};
use crate::module::{ModuleManager, ModuleWorkspace, UnsealedWorkspace};
use byteorder::{ByteOrder, LE};
use iced_x86::code_asm::CodeAssembler;
use std::error::Error;
use std::mem::transmute;
use std::ptr::null_mut;
use thiserror::Error;

/// An implementation of [`ExecutionEngine`] for running the PS4 binary natively.
pub struct NativeEngine<'a, 'b: 'a> {
    modules: &'a ModuleManager<'b>,
}

impl<'a, 'b: 'a> NativeEngine<'a, 'b> {
    pub fn new(modules: &'a ModuleManager<'b>) -> Self {
        Self { modules }
    }

    /// # SAFETY
    /// No other threads may read or write any module memory.
    pub unsafe fn patch_syscalls(&mut self) -> Result<(), PatchSyscallsError> {
        self.modules.for_each(|module| {
            let path: VPathBuf = module.image().name().try_into().unwrap();

            // Get the module memory.
            let mut mem = match module.memory().unprotect() {
                Ok(v) => v,
                Err(e) => return Err(PatchSyscallsError::UnprotectMemoryFailed(path, e)),
            };

            // Search for syscall pattern in all executable sections.
            let base = mem.addr();

            for seg in module.memory().segments() {
                if !seg.prot().contains(Protections::CPU_EXEC) {
                    continue;
                }

                // Patch segment.
                let start = seg.start();
                let mem = &mut mem[start..(start + seg.len())];

                Self::patch_syscalls_segment(&path, base, module.memory().workspace(), mem)?;
            }

            Ok(())
        })?;

        Ok(())
    }

    fn patch_syscalls_segment(
        module: &VPath,
        base: usize,
        wp: &ModuleWorkspace,
        seg: &mut [u8],
    ) -> Result<(), PatchSyscallsError> {
        let offset = seg.as_ptr() as usize - base;
        let mut next = 0;

        loop {
            // Looking for "mov rax, imm32".
            let start = match memchr::memmem::find(&seg[next..], &[0x48, 0xc7, 0xc0]) {
                Some(v) => v,
                None => break,
            };

            // Check if "mov rax, imm32".
            let mem = &mut seg[(next + start)..];

            if mem.len() < 7 {
                break;
            }

            // Check if the following instructions are:
            //
            // mov r10, rcx
            // syscall
            if !mem[7..].starts_with(&[0x49, 0x89, 0xca, 0x0f, 0x05]) {
                next += start + 3;
                continue;
            }

            // Build call trunk.
            let id = LE::read_u32(&mem[3..]);
            let ret = base + offset + next + start + 7 + 5;
            let offset = offset + next + start + 10;
            let trunk = match Self::build_syscall_trunk(wp.lock(), module, offset, id, ret) {
                Ok(v) => v,
                Err(e) => {
                    return Err(PatchSyscallsError::BuildTrunkFailed(
                        module.to_owned(),
                        offset,
                        e,
                    ))
                }
            };

            // Patch syscall.
            let trunk = match Self::get_relative_offset(ret, trunk) {
                Some(v) => v.to_ne_bytes(),
                None => return Err(PatchSyscallsError::WorkspaceTooFar(module.to_owned())),
            };

            mem[7] = 0xe9;
            mem[8] = trunk[0];
            mem[9] = trunk[1];
            mem[10] = trunk[2];
            mem[11] = trunk[3];

            next += start + 7 + 5;
        }

        Ok(())
    }

    fn build_syscall_trunk(
        mut wp: UnsealedWorkspace,
        module: &VPath,
        offset: usize,
        id: u32,
        ret: usize,
    ) -> Result<usize, SyscallTrunkError> {
        use iced_x86::code_asm::{rax, rbp, rdi, rdx, rsi, rsp};

        let mut asm = CodeAssembler::new(64).unwrap();

        // Create stack frame.
        asm.push(rbp).unwrap();
        asm.mov(rbp, rsp).unwrap();

        // Invoke the handler.
        let module = Box::new(module.to_owned());

        asm.mov(rdx, Box::into_raw(module) as u64).unwrap();
        asm.mov(rsi, offset as u64).unwrap();
        asm.mov(rdi, id as u64).unwrap();
        asm.and(rsp, !15).unwrap(); // Make sure stack is align to 16 bytes boundary.
        asm.mov(rax, Self::unimplemented_syscall as u64).unwrap();
        asm.call(rax).unwrap();

        // Restore registers.
        asm.leave().unwrap();

        // Align base address to 16 byte boundary for performance.
        let base = wp.addr();
        let start = match base % 16 {
            0 => 0,
            v => 16 - v,
        };

        // Check if workspace is enough.
        let output = match wp.as_mut_slice().get_mut(start..) {
            Some(v) => v,
            None => return Err(SyscallTrunkError::SpaceNotEnough),
        };

        // Assemble.
        let addr = base + start;
        let mut assembled = asm.assemble(addr as u64).unwrap();

        // Manually JMP rel32 back to patched location.
        assembled.extend({
            // Calculate relative offset.
            let mut code = [0u8; 5];
            let relative = match Self::get_relative_offset(addr + assembled.len() + code.len(), ret)
            {
                Some(v) => v,
                None => return Err(SyscallTrunkError::ReturnTooFar),
            };

            // Write the instruction.
            let v = relative.to_ne_bytes();

            code[0] = 0xe9;
            code[1] = v[0];
            code[2] = v[1];
            code[3] = v[2];
            code[4] = v[3];
            code
        });

        // Write the assembled code.
        if assembled.len() > output.len() {
            return Err(SyscallTrunkError::SpaceNotEnough);
        }

        output[..assembled.len()].copy_from_slice(&assembled);

        // Seal the workspace.
        wp.seal(start + assembled.len());

        Ok(addr)
    }

    fn get_relative_offset(from: usize, to: usize) -> Option<i32> {
        let offset = to.wrapping_sub(from) as isize;

        offset.try_into().ok()
    }

    extern "sysv64" fn unimplemented_syscall(id: u32, offset: usize, module: *mut VPathBuf) -> ! {
        let module = unsafe { Box::from_raw(module) };

        panic!("Syscall {id} is not implemented at {offset:#018x} on {module}.");
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

/// Errors for [`NativeEngine::patch_syscalls()`].
#[derive(Debug, Error)]
pub enum PatchSyscallsError {
    #[error("cannot unprotect the memory of {0}")]
    UnprotectMemoryFailed(VPathBuf, #[source] MprotectError),

    #[error("cannot build a trunk for syscall at {1:#018x} on {1}")]
    BuildTrunkFailed(VPathBuf, usize, #[source] SyscallTrunkError),

    #[error("workspace of {0} is too far")]
    WorkspaceTooFar(VPathBuf),
}

/// Errors for syscall trunk building.
#[derive(Debug, Error)]
pub enum SyscallTrunkError {
    #[error("the remaining workspace is not enough")]
    SpaceNotEnough,

    #[error("the address to return is too far")]
    ReturnTooFar,
}
